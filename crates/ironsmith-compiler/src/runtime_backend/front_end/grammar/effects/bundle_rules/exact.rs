use crate::runtime_backend::front_end::lexer::{OwnedLexToken, split_lexed_sentences};
use winnow::Parser as _;

use super::exact_surface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProliferatePhaseOutShape;

pub(crate) fn parse_proliferate_phase_out_pair_shape(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<ProliferatePhaseOutShape> {
    let first_words = crate::runtime_backend::front_end::lexer::parser_token_word_refs(first);
    let first = if first_words.first().copied() == Some("you") {
        crate::runtime_backend::front_end::grammar::primitives::parse_full_word_slice(
            &first_words,
            (
                crate::runtime_backend::front_end::grammar::primitives::word_slice_exact("you"),
                super::sequence(&[
                    "proliferate",
                    "then",
                    "choose",
                    "any",
                    "number",
                    "of",
                    "permanents",
                    "you",
                    "control",
                    "that",
                    "had",
                    "a",
                    "counter",
                    "put",
                    "on",
                    "them",
                    "this",
                    "way",
                ]),
            )
                .value(ProliferatePhaseOutShape),
        )
    } else {
        crate::runtime_backend::front_end::grammar::primitives::parse_full_word_slice(
            &first_words,
            super::sequence(&[
                "proliferate",
                "then",
                "choose",
                "any",
                "number",
                "of",
                "permanents",
                "you",
                "control",
                "that",
                "had",
                "a",
                "counter",
                "put",
                "on",
                "them",
                "this",
                "way",
            ])
            .value(ProliferatePhaseOutShape),
        )
    }?;
    exact_surface(second, &["those", "permanents", "phase", "out"]).then_some(first)
}

pub(crate) fn parse_proliferate_phase_out_single_shape(
    tokens: &[OwnedLexToken],
) -> Option<ProliferatePhaseOutShape> {
    let words = crate::runtime_backend::front_end::lexer::parser_token_word_refs(tokens);
    let tail = if words.first().copied() == Some("you") {
        words.get(1..)?
    } else {
        words.as_slice()
    };
    super::complete(
        tail,
        super::sequence(&[
            "proliferate",
            "then",
            "choose",
            "any",
            "number",
            "of",
            "permanents",
            "you",
            "control",
            "that",
            "had",
            "a",
            "counter",
            "put",
            "on",
            "them",
            "this",
            "way",
            "those",
            "permanents",
            "phase",
            "out",
        ])
        .value(ProliferatePhaseOutShape),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DrawTreasureLoseLifeShape;

pub(crate) fn parse_draw_treasure_lose_life_shape(
    tokens: &[OwnedLexToken],
) -> Option<DrawTreasureLoseLifeShape> {
    let words = crate::runtime_backend::front_end::lexer::parser_token_word_refs(tokens);
    let tail = if words.first().copied() == Some("you") {
        words.get(1..)?
    } else {
        words.as_slice()
    };
    super::complete(
        tail,
        super::sequence(&[
            "draw", "that", "many", "cards", "create", "that", "many", "tapped", "treasure",
            "tokens", "then", "lose", "that", "much", "life",
        ])
        .value(DrawTreasureLoseLifeShape),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KickedDoctorsReplacementShape;

pub(crate) fn parse_kicked_doctors_replacement_shape(
    tokens: &[OwnedLexToken],
) -> Option<KickedDoctorsReplacementShape> {
    let sentences = split_lexed_sentences(tokens);
    let [first, second, third] = sentences.as_slice() else {
        return None;
    };
    if !exact_surface(
        first,
        &[
            "search",
            "your",
            "library",
            "and",
            "or",
            "graveyard",
            "for",
            "up",
            "to",
            "five",
            "doctor",
            "cards",
            "reveal",
            "them",
            "and",
            "put",
            "them",
            "into",
            "your",
            "hand",
        ],
    ) || !exact_surface(
        second,
        &[
            "if", "you", "search", "your", "library", "this", "way", "shuffle",
        ],
    ) || !exact_surface(
        third,
        &[
            "if",
            "this",
            "spell",
            "was",
            "kicked",
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
    ) {
        return None;
    }
    Some(KickedDoctorsReplacementShape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SoulPartitionShape;

pub(crate) fn parse_soul_partition_shape(tokens: &[OwnedLexToken]) -> Option<SoulPartitionShape> {
    let sentences = split_lexed_sentences(tokens);
    let [first, second, third] = sentences.as_slice() else {
        return None;
    };
    if !exact_surface(first, &["exile", "target", "nonland", "permanent"])
        || !exact_surface(
            second,
            &[
                "for", "as", "long", "as", "that", "card", "remains", "exiled", "its", "owner",
                "may", "play", "it",
            ],
        )
        || !(exact_surface(
            third,
            &[
                "a", "spell", "cast", "by", "an", "opponent", "this", "way", "costs", "2", "more",
                "to", "cast",
            ],
        ) || exact_surface(
            third,
            &[
                "a", "spell", "cast", "by", "an", "opponent", "this", "way", "costs", "{2}",
                "more", "to", "cast",
            ],
        ))
    {
        return None;
    }
    Some(SoulPartitionShape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EmptyLaboratoryShape;

pub(crate) fn parse_empty_laboratory_shape(
    tokens: &[OwnedLexToken],
) -> Option<EmptyLaboratoryShape> {
    exact_surface(
        tokens,
        &[
            "sacrifice",
            "x",
            "zombies",
            "then",
            "reveal",
            "cards",
            "from",
            "the",
            "top",
            "of",
            "your",
            "library",
            "until",
            "you",
            "reveal",
            "a",
            "number",
            "of",
            "zombie",
            "creature",
            "cards",
            "equal",
            "to",
            "the",
            "number",
            "of",
            "zombies",
            "sacrificed",
            "this",
            "way",
            "put",
            "those",
            "cards",
            "onto",
            "the",
            "battlefield",
            "and",
            "the",
            "rest",
            "on",
            "the",
            "bottom",
            "of",
            "your",
            "library",
            "in",
            "a",
            "random",
            "order",
        ],
    )
    .then_some(EmptyLaboratoryShape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShapeAnewShape;

pub(crate) fn parse_shape_anew_shape(tokens: &[OwnedLexToken]) -> Option<ShapeAnewShape> {
    exact_surface(
        tokens,
        &[
            "the",
            "controller",
            "of",
            "target",
            "artifact",
            "sacrifices",
            "it",
            "then",
            "reveals",
            "cards",
            "from",
            "the",
            "top",
            "of",
            "their",
            "library",
            "until",
            "they",
            "reveal",
            "an",
            "artifact",
            "card",
            "that",
            "player",
            "puts",
            "that",
            "card",
            "onto",
            "the",
            "battlefield",
            "then",
            "shuffles",
            "all",
            "other",
            "cards",
            "revealed",
            "this",
            "way",
            "into",
            "their",
            "library",
        ],
    )
    .then_some(ShapeAnewShape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TapLandsEmptyManaShape;

pub(crate) fn parse_tap_lands_empty_mana_shape(
    tokens: &[OwnedLexToken],
) -> Option<TapLandsEmptyManaShape> {
    exact_surface(
        tokens,
        &[
            "tap", "all", "lands", "target", "player", "controls", "and", "that", "player",
            "loses", "all", "unspent", "mana",
        ],
    )
    .then_some(TapLandsEmptyManaShape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CollisionOfRealmsShape;

pub(crate) fn parse_collision_of_realms_shape(
    tokens: &[OwnedLexToken],
) -> Option<CollisionOfRealmsShape> {
    exact_surface(
        tokens,
        &[
            "each",
            "player",
            "shuffles",
            "all",
            "creatures",
            "they",
            "own",
            "into",
            "their",
            "library",
            "each",
            "player",
            "who",
            "shuffled",
            "a",
            "nontoken",
            "creature",
            "into",
            "their",
            "library",
            "this",
            "way",
            "reveals",
            "cards",
            "from",
            "the",
            "top",
            "of",
            "their",
            "library",
            "until",
            "they",
            "reveal",
            "a",
            "creature",
            "card",
            "then",
            "puts",
            "that",
            "card",
            "onto",
            "the",
            "battlefield",
            "and",
            "the",
            "rest",
            "on",
            "the",
            "bottom",
            "of",
            "their",
            "library",
            "in",
            "a",
            "random",
            "order",
        ],
    )
    .then_some(CollisionOfRealmsShape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NissasEncouragementShape;

pub(crate) fn parse_nissas_encouragement_shape(
    tokens: &[OwnedLexToken],
) -> Option<NissasEncouragementShape> {
    exact_surface(
        tokens,
        &[
            "search",
            "your",
            "library",
            "and",
            "graveyard",
            "for",
            "a",
            "card",
            "named",
            "forest",
            "a",
            "card",
            "named",
            "brambleweft",
            "behemoth",
            "and",
            "a",
            "card",
            "named",
            "nissa",
            "genesis",
            "mage",
            "reveal",
            "those",
            "cards",
            "put",
            "them",
            "into",
            "your",
            "hand",
            "then",
            "shuffle",
        ],
    )
    .then_some(NissasEncouragementShape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EachPlayerBounceDrawShape;

pub(crate) fn parse_each_player_bounce_draw_shape(
    tokens: &[OwnedLexToken],
) -> Option<EachPlayerBounceDrawShape> {
    let sentences = split_lexed_sentences(tokens);
    let [choose, bounce, draw] = sentences.as_slice() else {
        return None;
    };
    let bounce_words = crate::runtime_backend::front_end::lexer::parser_token_word_refs(bounce);
    let bounce_tail = super::consume_head(
        &bounce_words,
        &[
            "return",
            "all",
            "nonland",
            "permanents",
            "not",
            "chosen",
            "this",
            "way",
            "to",
            "their",
        ],
    )?;
    if !exact_surface(
        choose,
        &[
            "each",
            "player",
            "chooses",
            "a",
            "nonland",
            "permanent",
            "they",
            "control",
        ],
    ) || bounce_tail.last().copied() != Some("hands")
        || !exact_surface(
            draw,
            &[
                "then", "you", "draw", "a", "card", "for", "each", "opponent", "who", "has",
                "more", "cards", "in", "their", "hand", "than", "you",
            ],
        )
    {
        return None;
    }
    Some(EachPlayerBounceDrawShape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpecialExactBundleShape {
    ThassasOracle,
    GeistblastFromGraveyard,
}

pub(crate) fn parse_special_exact_bundle_shape(
    tokens: &[OwnedLexToken],
) -> Option<SpecialExactBundleShape> {
    if exact_surface(
        tokens,
        &[
            "look", "at", "the", "top", "x", "cards", "of", "your", "library", "where", "x", "is",
            "your", "devotion", "to", "blue", "put", "up", "to", "one", "of", "them", "on", "top",
            "of", "your", "library", "and", "the", "rest", "on", "the", "bottom", "of", "your",
            "library", "in", "a", "random", "order", "if", "x", "is", "greater", "than", "or",
            "equal", "to", "the", "number", "of", "cards", "in", "your", "library", "you", "win",
            "the", "game",
        ],
    ) {
        return Some(SpecialExactBundleShape::ThassasOracle);
    }
    exact_surface(
        tokens,
        &[
            "if",
            "this",
            "spell",
            "was",
            "cast",
            "from",
            "a",
            "graveyard",
            "copy",
            "this",
            "spell",
            "and",
            "you",
            "may",
            "choose",
            "a",
            "new",
            "target",
            "for",
            "the",
            "copy",
        ],
    )
    .then_some(SpecialExactBundleShape::GeistblastFromGraveyard)
}
