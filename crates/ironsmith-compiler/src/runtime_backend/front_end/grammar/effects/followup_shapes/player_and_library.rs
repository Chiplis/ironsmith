use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LibraryShuffleFollowupShape {
    IfSearchedThisWay,
    ThatPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DamagedPlayerFollowupShape {
    CantCastNoncreatureSpellsThisTurn,
    CantGainLifeRestOfGame,
}

fn library_shuffle_followup<'a>(input: &mut LexStream<'a>) -> WResult<LibraryShuffleFollowupShape> {
    alt((
        (
            primitives::phrase(&["if", "you", "search", "your", "library", "this", "way"]),
            opt(primitives::comma()),
            alt((primitives::kw("shuffle"), primitives::kw("shuffles"))),
            primitives::sentence_end(),
        )
            .value(LibraryShuffleFollowupShape::IfSearchedThisWay),
        (
            opt(primitives::kw("then")),
            primitives::phrase(&["that", "player"]),
            alt((primitives::kw("shuffle"), primitives::kw("shuffles"))),
            primitives::sentence_end(),
        )
            .value(LibraryShuffleFollowupShape::ThatPlayer),
    ))
    .parse_next(input)
}

pub(crate) fn parse_library_shuffle_followup_shape(
    tokens: &[OwnedLexToken],
) -> Option<LibraryShuffleFollowupShape> {
    primitives::parse_all(tokens, library_shuffle_followup, "library shuffle followup").ok()
}

fn cant<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("cant"), primitives::kw("can't")))
        .void()
        .parse_next(input)
}

fn damaged_player_followup<'a>(input: &mut LexStream<'a>) -> WResult<DamagedPlayerFollowupShape> {
    alt((
        (
            primitives::phrase(&["players", "dealt", "damage", "this", "way"]),
            opt(primitives::comma()),
            cant,
            primitives::kw("cast"),
            alt((
                primitives::kw("noncreature").void(),
                primitives::phrase(&["non", "creature"]).void(),
            )),
            primitives::phrase(&["spells", "this", "turn"]),
            primitives::sentence_end(),
        )
            .value(DamagedPlayerFollowupShape::CantCastNoncreatureSpellsThisTurn),
        (
            primitives::kw("if"),
            opt(primitives::kw("a")),
            primitives::phrase(&["player", "is", "dealt", "damage", "this", "way"]),
            opt(primitives::comma()),
            primitives::kw("they"),
            cant,
            primitives::phrase(&["gain", "life", "for", "the", "rest", "of", "the", "game"]),
            primitives::sentence_end(),
        )
            .value(DamagedPlayerFollowupShape::CantGainLifeRestOfGame),
    ))
    .parse_next(input)
}

pub(crate) fn parse_damaged_player_followup_shape(
    tokens: &[OwnedLexToken],
) -> Option<DamagedPlayerFollowupShape> {
    primitives::parse_all(tokens, damaged_player_followup, "damaged player followup").ok()
}

pub(crate) fn is_tap_damaged_creatures_followup(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            primitives::kw("tap"),
            alt((primitives::kw("each"), primitives::kw("all"))),
            alt((primitives::kw("creature"), primitives::kw("creatures"))),
            primitives::phrase(&["dealt", "damage", "this", "way"]),
            primitives::sentence_end(),
        )
            .void(),
        "tap damaged creatures followup",
    )
    .is_ok()
}

pub(crate) fn is_still_land_followup(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            alt((
                primitives::phrase(&["they're", "still"]),
                primitives::phrase(&["theyre", "still"]),
                primitives::phrase(&["they", "re", "still"]),
                primitives::phrase(&["it's", "still"]),
                primitives::phrase(&["its", "still"]),
                primitives::phrase(&["it", "s", "still"]),
            )),
            opt(primitives::kw("a")),
            alt((primitives::kw("land"), primitives::kw("lands"))),
            primitives::sentence_end(),
        )
            .void(),
        "still land followup",
    )
    .is_ok()
}

pub(crate) fn is_destroy_those_creatures_followup(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            opt(primitives::kw("then")),
            primitives::phrase(&["destroy", "those", "creatures"]),
            primitives::sentence_end(),
        )
            .void(),
        "destroy those creatures followup",
    )
    .is_ok()
}
