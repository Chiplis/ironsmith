use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::PlayerAst;
use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, parser_token_word_refs, trim_lexed_commas};
use crate::target::PlayerFilter;
use crate::util::possessive_normalized_word_refs;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchZonePairShape {
    pub owner: PlayerFilter,
    pub first_zone: Zone,
    pub second_zone: Zone,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchTargetExileBundleShape {
    pub player: PlayerAst,
    pub filter: PlayerFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchForEachWayKind {
    Exiled,
    DestroyedOrDied,
    PutIntoGraveyard,
    Sacrificed,
}

#[derive(Debug, Clone, Copy)]
pub struct SearchForEachWayShape<'a> {
    pub kind: SearchForEachWayKind,
    pub iterated_filter_tokens: Option<&'a [OwnedLexToken]>,
    pub effect_tokens: Option<&'a [OwnedLexToken]>,
    pub permanent_card_type_consult: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchExiledConsultFinish {
    Shuffle,
    PutRestOnBottom,
}

#[derive(Debug, Clone, Copy)]
pub struct SearchExiledConsultShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub finish: SearchExiledConsultFinish,
}

fn word(input: &mut primitives::WordSliceInput<'_>, expected: &'static str) -> WResult<()> {
    primitives::word_slice_exact(expected)
        .void()
        .parse_next(input)
}

fn zone_word(input: &mut primitives::WordSliceInput<'_>) -> WResult<Zone> {
    alt((
        alt((
            primitives::word_slice_exact("hand"),
            primitives::word_slice_exact("hands"),
        ))
        .value(Zone::Hand),
        alt((
            primitives::word_slice_exact("graveyard"),
            primitives::word_slice_exact("graveyards"),
        ))
        .value(Zone::Graveyard),
    ))
    .parse_next(input)
}

fn zone_pair_owner(input: &mut primitives::WordSliceInput<'_>) -> WResult<PlayerFilter> {
    alt((
        (
            primitives::word_slice_exact("target"),
            alt((
                primitives::word_slice_exact("player"),
                primitives::word_slice_exact("players"),
            )),
        )
            .value(PlayerFilter::target_player()),
        (
            primitives::word_slice_exact("target"),
            alt((
                primitives::word_slice_exact("opponent"),
                primitives::word_slice_exact("opponents"),
            )),
        )
            .value(PlayerFilter::target_opponent()),
        primitives::word_slice_exact("your").value(PlayerFilter::You),
    ))
    .parse_next(input)
}

fn parse_zone_pair_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<SearchZonePairShape> {
    (
        primitives::word_slice_exact("exile"),
        primitives::word_slice_exact("all"),
        primitives::word_slice_exact("cards"),
        primitives::word_slice_exact("from"),
    )
        .void()
        .parse_next(input)?;
    let owner = zone_pair_owner.parse_next(input)?;
    let first_zone = zone_word.parse_next(input)?;
    word(input, "and")?;
    let _: () = repeat(
        0..,
        alt((
            primitives::word_slice_exact("all"),
            primitives::word_slice_exact("cards"),
            primitives::word_slice_exact("from"),
        ))
        .void(),
    )
    .parse_next(input)?;
    let second_zone = zone_word.parse_next(input)?;
    eof.parse_next(input)?;
    if first_zone == second_zone {
        return Err(primitives::backtrack_err(
            "zone pair",
            "two different zones",
        ));
    }
    Ok(SearchZonePairShape {
        owner,
        first_zone,
        second_zone,
    })
}

pub fn parse_search_exile_zone_pair_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<SearchZonePairShape> {
    let words = parser_token_word_refs(tokens);
    let normalized = possessive_normalized_word_refs(&words);
    let mut input: primitives::WordSliceInput<'_> = &normalized;
    parse_zone_pair_words.parse_next(&mut input).ok()
}

fn target_exile_subject(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<SearchTargetExileBundleShape> {
    word(input, "target")?;
    let opponent = alt((
        alt((
            primitives::word_slice_exact("opponent"),
            primitives::word_slice_exact("opponents"),
        ))
        .value(true),
        alt((
            primitives::word_slice_exact("player"),
            primitives::word_slice_exact("players"),
        ))
        .value(false),
    ))
    .parse_next(input)?;
    Ok(if opponent {
        SearchTargetExileBundleShape {
            player: PlayerAst::TargetOpponent,
            filter: PlayerFilter::target_opponent(),
        }
    } else {
        SearchTargetExileBundleShape {
            player: PlayerAst::Target,
            filter: PlayerFilter::target_player(),
        }
    })
}

fn parse_target_exile_bundle_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<SearchTargetExileBundleShape> {
    let subject = target_exile_subject.parse_next(input)?;
    alt((
        primitives::word_slice_exact("exile"),
        primitives::word_slice_exact("exiles"),
    ))
    .void()
    .parse_next(input)?;
    opt(alt((
        primitives::word_slice_exact("a"),
        primitives::word_slice_exact("an"),
        primitives::word_slice_exact("the"),
    )))
    .parse_next(input)?;
    word(input, "creature")?;
    alt((
        (
            primitives::word_slice_exact("they"),
            alt((
                primitives::word_slice_exact("control"),
                primitives::word_slice_exact("controls"),
            )),
        )
            .void(),
        (
            primitives::word_slice_exact("that"),
            primitives::word_slice_exact("player"),
            alt((
                primitives::word_slice_exact("control"),
                primitives::word_slice_exact("controls"),
            )),
        )
            .void(),
    ))
    .parse_next(input)?;
    word(input, "and")?;
    word(input, "their")?;
    alt((
        primitives::word_slice_exact("graveyard"),
        primitives::word_slice_exact("graveyards"),
    ))
    .void()
    .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(subject)
}

pub fn parse_target_exile_bundle_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<SearchTargetExileBundleShape> {
    let words = parser_token_word_refs(trim_lexed_commas(tokens));
    let mut input: primitives::WordSliceInput<'_> = &words;
    parse_target_exile_bundle_words.parse_next(&mut input).ok()
}

fn marker_present(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some()
}

fn word_present(tokens: &[OwnedLexToken], expected: &'static str) -> bool {
    primitives::find_prefix(tokens, || primitives::kw(expected)).is_some()
}

fn comma_parts(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], Option<&[OwnedLexToken]>) {
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_comma() {
            return (
                trim_lexed_commas(&tokens[..idx]),
                Some(trim_lexed_commas(&tokens[idx + 1..])),
            );
        }
    }
    (trim_lexed_commas(tokens), None)
}

pub fn parse_search_for_each_way_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<SearchForEachWayShape<'_>> {
    let (head, effect_tokens) = comma_parts(tokens);
    let (_, after_for_each) = primitives::parse_prefix(head, primitives::phrase(&["for", "each"]))?;
    let sacrificed_marker = primitives::find_prefix(after_for_each, || {
        primitives::phrase(&["sacrificed", "this", "way"])
    });
    let exiled_marker = primitives::find_prefix(after_for_each, || {
        primitives::phrase(&["exiled", "this", "way"])
    });
    let destroyed_or_died_marker = primitives::find_prefix(after_for_each, || {
        alt((
            primitives::phrase(&["destroyed", "this", "way"]),
            primitives::phrase(&["died", "this", "way"]),
        ))
    });
    let put_into_graveyard_marker = primitives::find_prefix(after_for_each, || {
        alt((
            primitives::phrase(&["put", "into", "a", "graveyard", "this", "way"]),
            primitives::phrase(&["put", "into", "graveyard", "this", "way"]),
            primitives::phrase(&["put", "into", "their", "graveyard", "this", "way"]),
            primitives::phrase(&["put", "into", "its", "graveyard", "this", "way"]),
        ))
    });
    let kind = if exiled_marker.is_some()
        || primitives::parse_prefix(
            head,
            primitives::phrase(&["for", "each", "of", "those", "creatures"]),
        )
        .is_some()
    {
        SearchForEachWayKind::Exiled
    } else if destroyed_or_died_marker.is_some() {
        SearchForEachWayKind::DestroyedOrDied
    } else if put_into_graveyard_marker.is_some() {
        SearchForEachWayKind::PutIntoGraveyard
    } else if sacrificed_marker.is_some() {
        SearchForEachWayKind::Sacrificed
    } else {
        return None;
    };
    let iterated_filter_tokens = match kind {
        SearchForEachWayKind::DestroyedOrDied => destroyed_or_died_marker,
        SearchForEachWayKind::Sacrificed => sacrificed_marker,
        SearchForEachWayKind::Exiled => exiled_marker,
        SearchForEachWayKind::PutIntoGraveyard => put_into_graveyard_marker,
    }
    .map(|(marker_idx, _, _)| trim_lexed_commas(&after_for_each[..marker_idx]));
    let permanent_card_type_consult = kind == SearchForEachWayKind::Exiled
        && primitives::parse_prefix(
            head,
            primitives::phrase(&["for", "each", "permanent", "exiled", "this", "way"]),
        )
        .is_some()
        && word_present(tokens, "shares")
        && word_present(tokens, "card")
        && word_present(tokens, "type")
        && word_present(tokens, "library")
        && word_present(tokens, "battlefield")
        && !word_present(tokens, "shuffles");
    Some(SearchForEachWayShape {
        kind,
        iterated_filter_tokens,
        effect_tokens,
        permanent_card_type_consult,
    })
}

pub fn parse_search_exiled_consult_shape_lexed(
    effect_tokens: &[OwnedLexToken],
) -> Option<SearchExiledConsultShape<'_>> {
    let (_, after_prefix) = primitives::parse_prefix(
        effect_tokens,
        primitives::phrase(&[
            "its",
            "controller",
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
        ]),
    )?;
    let (put_start, (), after_put) = primitives::find_prefix(after_prefix, || {
        primitives::phrase(&["puts", "that", "card", "onto", "the", "battlefield"]).void()
    })?;
    let filter_tokens = trim_lexed_commas(&after_prefix[..put_start]);
    if filter_tokens.is_empty() {
        return None;
    }
    let finish = if marker_present(after_put, &["then", "shuffles"]) {
        SearchExiledConsultFinish::Shuffle
    } else if marker_present(
        after_put,
        &["then", "puts", "the", "rest", "on", "the", "bottom"],
    ) {
        SearchExiledConsultFinish::PutRestOnBottom
    } else {
        return None;
    };
    Some(SearchExiledConsultShape {
        filter_tokens,
        finish,
    })
}

pub fn search_each_player_exiled_permanents_shape_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        primitives::phrase(&["each", "player", "turns", "face", "up", "all", "cards"]),
    )
    .is_some()
        && marker_present(tokens, &["exiled", "with", "this"])
        && marker_present(tokens, &["then", "puts", "all", "permanent", "cards"])
        && (marker_present(tokens, &["among", "them", "onto", "battlefield"])
            || marker_present(tokens, &["among", "them", "onto", "the", "battlefield"]))
}

#[cfg(test)]
mod tests {
    use crate::lexer::lex_line;

    use super::*;

    #[test]
    fn parses_zone_pair_and_for_each_shapes() {
        let tokens =
            lex_line("Exile all cards from target player's hand and graveyard", 0).unwrap();
        let pair = parse_search_exile_zone_pair_shape_lexed(&tokens).unwrap();
        assert_eq!(pair.first_zone, Zone::Hand);
        assert_eq!(pair.second_zone, Zone::Graveyard);

        let tokens = lex_line(
            "For each permanent destroyed this way, its controller draws a card",
            0,
        )
        .unwrap();
        let shape = parse_search_for_each_way_shape_lexed(&tokens).unwrap();
        assert_eq!(shape.kind, SearchForEachWayKind::DestroyedOrDied);
        assert_eq!(
            parser_token_word_refs(shape.iterated_filter_tokens.unwrap()),
            vec!["permanent"]
        );
        assert!(!shape.effect_tokens.unwrap().is_empty());

        let tokens = lex_line(
            "For each nontoken creature destroyed this way, you create a Treasure token",
            0,
        )
        .unwrap();
        let shape = parse_search_for_each_way_shape_lexed(&tokens).unwrap();
        assert_eq!(shape.kind, SearchForEachWayKind::DestroyedOrDied);
        assert_eq!(
            parser_token_word_refs(shape.iterated_filter_tokens.unwrap()),
            vec!["nontoken", "creature"]
        );

        let tokens =
            lex_line("For each creature card exiled this way, you gain 1 life", 0).unwrap();
        let shape = parse_search_for_each_way_shape_lexed(&tokens).unwrap();
        assert_eq!(shape.kind, SearchForEachWayKind::Exiled);
        assert_eq!(
            parser_token_word_refs(shape.iterated_filter_tokens.unwrap()),
            vec!["creature", "card"]
        );

        let tokens = lex_line(
            "For each land sacrificed this way, its controller may search their library",
            0,
        )
        .unwrap();
        let shape = parse_search_for_each_way_shape_lexed(&tokens).unwrap();
        assert_eq!(shape.kind, SearchForEachWayKind::Sacrificed);
        assert_eq!(
            parser_token_word_refs(shape.iterated_filter_tokens.unwrap()),
            vec!["land"]
        );

        let tokens = lex_line(
            "For each creature card put into a graveyard this way, you create a Zombie token",
            0,
        )
        .unwrap();
        let shape = parse_search_for_each_way_shape_lexed(&tokens).unwrap();
        assert_eq!(shape.kind, SearchForEachWayKind::PutIntoGraveyard);
        assert_eq!(
            parser_token_word_refs(shape.iterated_filter_tokens.unwrap()),
            vec!["creature", "card"]
        );
    }
}
