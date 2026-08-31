use winnow::combinator::{alt, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::grammar::{filters, primitives};
use crate::lexer::{LexStream, OwnedLexToken, split_lexed_sentences, trim_lexed_commas};
use crate::target::ObjectFilter;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ControllerSacrificeConsultShape {
    pub target_filter: ObjectFilter,
    pub match_filter: ObjectFilter,
    pub destination: Zone,
    pub conditional_on_sacrifice: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EachPlayerShuffleThenConsultShape {
    pub shuffled_filter: ObjectFilter,
    pub qualifying_filter: ObjectFilter,
    pub match_filter: ObjectFilter,
    pub destination: Zone,
    pub remainder_order: LibraryBottomOrderAst,
}

fn commas<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., primitives::comma().void()).parse_next(input)
}

fn zone<'a>(input: &mut LexStream<'a>) -> WResult<Zone> {
    alt((
        primitives::kw("battlefield").value(Zone::Battlefield),
        primitives::kw("hand").value(Zone::Hand),
        primitives::kw("graveyard").value(Zone::Graveyard),
        primitives::kw("exile").value(Zone::Exile),
        primitives::kw("library").value(Zone::Library),
    ))
    .parse_next(input)
}

fn controller_consult_head<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    commas(input)?;
    opt(primitives::kw("then")).parse_next(input)?;
    alt((primitives::kw("reveals"), primitives::kw("reveal"))).parse_next(input)?;
    primitives::phrase(&[
        "cards", "from", "the", "top", "of", "their", "library", "until",
    ])
    .parse_next(input)?;
    alt((
        primitives::phrase(&["they", "reveal"]),
        primitives::phrase(&["that", "player", "reveals"]),
    ))
    .void()
    .parse_next(input)
}

fn controller_consult_followup<'a>(input: &mut LexStream<'a>) -> WResult<Zone> {
    primitives::phrase(&["that", "player", "puts", "that", "card"]).parse_next(input)?;
    alt((primitives::kw("onto"), primitives::kw("into"))).parse_next(input)?;
    opt(alt((primitives::kw("the"), primitives::kw("their")))).parse_next(input)?;
    let destination = zone.parse_next(input)?;
    commas(input)?;
    opt(primitives::kw("then")).parse_next(input)?;
    primitives::phrase(&[
        "shuffles", "all", "other", "cards", "revealed", "this", "way", "into", "their", "library",
    ])
    .parse_next(input)?;
    Ok(destination)
}

fn parse_legacy_controller_sacrifice_consult_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ControllerSacrificeConsultShape> {
    let sentences = split_lexed_sentences(tokens);
    let [sacrifice_and_consult, followup] = sentences.as_slice() else {
        return None;
    };
    let (_, target_and_tail) = primitives::parse_prefix(
        sacrifice_and_consult,
        primitives::phrase(&["the", "controller", "of", "target"]).void(),
    )?;
    let (target_tokens, consult_tokens) =
        primitives::split_lexed_once_on_separator(target_and_tail, || {
            primitives::phrase(&["sacrifices", "it"]).void()
        })?;
    let mut target_filter = crate::grammar::primitives::probe_shape(
        filters::parse_object_filter_with_grammar_entrypoint_lexed(
            trim_lexed_commas(target_tokens),
            false,
        ),
    )?;
    target_filter.zone = Some(Zone::Battlefield);
    let (_, match_tokens) =
        primitives::parse_prefix(trim_lexed_commas(consult_tokens), controller_consult_head)?;
    let mut match_filter = crate::grammar::primitives::probe_shape(
        filters::parse_object_filter_with_grammar_entrypoint_lexed(
            trim_lexed_commas(match_tokens),
            false,
        ),
    )?;
    match_filter.zone = None;
    let destination = crate::grammar::primitives::probe_shape(primitives::parse_all_or_none(
        trim_lexed_commas(followup),
        controller_consult_followup,
        "controller sacrifice consult followup",
    ))
    .flatten()?;
    Some(ControllerSacrificeConsultShape {
        target_filter,
        match_filter,
        destination,
        conditional_on_sacrifice: false,
    })
}

fn conditional_controller_consult_head<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "the", "player", "does"]).parse_next(input)?;
    commas(input)?;
    primitives::phrase(&[
        "they", "reveal", "cards", "from", "the", "top", "of", "their", "library", "until", "they",
        "reveal",
    ])
    .void()
    .parse_next(input)
}

fn conditional_controller_consult_followup<'a>(input: &mut LexStream<'a>) -> WResult<Zone> {
    alt((primitives::kw("onto"), primitives::kw("into"))).parse_next(input)?;
    opt(alt((primitives::kw("the"), primitives::kw("their")))).parse_next(input)?;
    let destination = zone.parse_next(input)?;
    commas(input)?;
    opt(primitives::kw("then")).parse_next(input)?;
    primitives::kw("shuffle").parse_next(input)?;
    Ok(destination)
}

fn normalize_possessive_filter_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    let Some(last) = normalized.last_mut() else {
        return normalized;
    };
    let Some(word) = last.as_word() else {
        return normalized;
    };
    let stem = crate::util::strip_possessive_suffix(word).to_string();
    if stem != word {
        last.replace_word(stem);
    }
    normalized
}

fn parse_conditional_controller_sacrifice_consult_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ControllerSacrificeConsultShape> {
    let sentences = split_lexed_sentences(tokens);
    let [sacrifice, conditional_consult] = sentences.as_slice() else {
        return None;
    };

    let (_, target_and_tail) =
        primitives::parse_prefix(sacrifice, primitives::kw("target").void())?;
    let (target_tokens, sacrifice_tail) =
        primitives::split_lexed_once_on_separator(target_and_tail, || {
            primitives::phrase(&["controller", "sacrifices", "it"]).void()
        })?;
    if !trim_lexed_commas(sacrifice_tail).is_empty() {
        return None;
    }
    let target_tokens = normalize_possessive_filter_tokens(trim_lexed_commas(target_tokens));
    let mut target_filter = crate::grammar::primitives::probe_shape(
        filters::parse_object_filter_with_grammar_entrypoint_lexed(&target_tokens, false),
    )?;
    target_filter.zone = Some(Zone::Battlefield);

    let (_, match_and_followup) =
        primitives::parse_prefix(conditional_consult, conditional_controller_consult_head)?;
    let (match_tokens, followup_tokens) =
        primitives::split_lexed_once_on_separator(match_and_followup, || {
            primitives::phrase(&["put", "that", "card"]).void()
        })?;
    let mut match_filter = crate::grammar::primitives::probe_shape(
        filters::parse_object_filter_with_grammar_entrypoint_lexed(
            trim_lexed_commas(match_tokens),
            false,
        ),
    )?;
    match_filter.zone = None;
    let destination = crate::grammar::primitives::probe_shape(primitives::parse_all_or_none(
        trim_lexed_commas(followup_tokens),
        conditional_controller_consult_followup,
        "conditional controller sacrifice consult followup",
    ))
    .flatten()?;

    Some(ControllerSacrificeConsultShape {
        target_filter,
        match_filter,
        destination,
        conditional_on_sacrifice: true,
    })
}

pub fn parse_controller_sacrifice_consult_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ControllerSacrificeConsultShape> {
    parse_legacy_controller_sacrifice_consult_tokens(tokens)
        .or_else(|| parse_conditional_controller_sacrifice_consult_tokens(tokens))
}

fn each_player_shuffle_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["they", "own", "into", "their", "library"])
        .void()
        .parse_next(input)
}

fn qualifying_shuffle_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["into", "their", "library", "this", "way"])
        .void()
        .parse_next(input)
}

fn each_player_consult_head<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("reveals"), primitives::kw("reveal"))).parse_next(input)?;
    primitives::phrase(&[
        "cards", "from", "the", "top", "of", "their", "library", "until",
    ])
    .parse_next(input)?;
    alt((
        primitives::phrase(&["they", "reveal"]),
        primitives::phrase(&["that", "player", "reveals"]),
    ))
    .void()
    .parse_next(input)
}

fn random_bottom_disposition<'a>(input: &mut LexStream<'a>) -> WResult<Zone> {
    opt(alt((primitives::kw("the"), primitives::kw("their")))).parse_next(input)?;
    let destination = zone.parse_next(input)?;
    primitives::phrase(&[
        "and", "the", "rest", "on", "the", "bottom", "of", "their", "library", "in", "a", "random",
        "order",
    ])
    .parse_next(input)?;
    Ok(destination)
}

pub fn parse_each_player_shuffle_then_consult_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EachPlayerShuffleThenConsultShape> {
    let sentences = split_lexed_sentences(tokens);
    let [shuffle, consult] = sentences.as_slice() else {
        return None;
    };
    let (_, shuffled_tail) = primitives::parse_prefix(
        shuffle,
        primitives::phrase(&["each", "player", "shuffles", "all"]).void(),
    )?;
    let (shuffled_tokens, shuffled_suffix) =
        primitives::split_lexed_once_on_separator(shuffled_tail, || each_player_shuffle_suffix)?;
    if !trim_lexed_commas(shuffled_suffix).is_empty() {
        return None;
    }
    let mut shuffled_filter = crate::grammar::primitives::probe_shape(
        filters::parse_object_filter_with_grammar_entrypoint_lexed(
            trim_lexed_commas(shuffled_tokens),
            false,
        ),
    )?;
    shuffled_filter.zone = Some(Zone::Battlefield);

    let (_, qualifying_tail) = primitives::parse_prefix(
        consult,
        primitives::phrase(&["each", "player", "who", "shuffled"]).void(),
    )?;
    let (qualifying_tokens, consult_tail) =
        primitives::split_lexed_once_on_separator(qualifying_tail, || qualifying_shuffle_suffix)?;
    let mut qualifying_filter = crate::grammar::primitives::probe_shape(
        filters::parse_object_filter_with_grammar_entrypoint_lexed(
            trim_lexed_commas(qualifying_tokens),
            false,
        ),
    )?;
    qualifying_filter.zone = Some(Zone::Battlefield);
    let (_, match_and_disposition) =
        primitives::parse_prefix(trim_lexed_commas(consult_tail), each_player_consult_head)?;
    let (match_tokens, disposition_tokens) =
        primitives::split_lexed_once_on_separator(match_and_disposition, || {
            primitives::phrase(&["then", "puts", "that", "card", "onto"]).void()
        })?;
    let mut match_filter = crate::grammar::primitives::probe_shape(
        filters::parse_object_filter_with_grammar_entrypoint_lexed(
            trim_lexed_commas(match_tokens),
            false,
        ),
    )?;
    match_filter.zone = None;
    let destination = crate::grammar::primitives::probe_shape(primitives::parse_all_or_none(
        trim_lexed_commas(disposition_tokens),
        random_bottom_disposition,
        "each-player consult disposition",
    ))
    .flatten()?;
    Some(EachPlayerShuffleThenConsultShape {
        shuffled_filter,
        qualifying_filter,
        match_filter,
        destination,
        remainder_order: LibraryBottomOrderAst::Random,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    use crate::types::CardType;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_controller_sacrifice_consult_for_arbitrary_card_type() {
        let shape = parse_controller_sacrifice_consult_tokens(&lex(
            "The controller of target enchantment sacrifices it, then reveals cards from the top of their library until they reveal an enchantment card. That player puts that card onto the battlefield, then shuffles all other cards revealed this way into their library.",
        ))
        .unwrap();
        assert!(
            shape
                .target_filter
                .card_types
                .contains(&CardType::Enchantment)
        );
        assert!(
            shape
                .match_filter
                .card_types
                .contains(&CardType::Enchantment)
        );
        assert!(!shape.conditional_on_sacrifice);
    }

    #[test]
    fn parses_conditional_controller_sacrifice_consult_with_shared_type_relation() {
        let shape = parse_controller_sacrifice_consult_tokens(&lex(
            "Target artifact's controller sacrifices it. If the player does, they reveal cards from the top of their library until they reveal an artifact card that shares a card type with the sacrificed artifact, put that card onto the battlefield, then shuffle.",
        ))
        .unwrap();
        assert!(shape.target_filter.card_types.contains(&CardType::Artifact));
        assert!(shape.match_filter.card_types.contains(&CardType::Artifact));
        assert!(shape.conditional_on_sacrifice);
        assert_eq!(shape.destination, Zone::Battlefield);
    }

    #[test]
    fn parses_each_player_shuffle_then_conditional_consult_generically() {
        let shape = parse_each_player_shuffle_then_consult_tokens(&lex(
            "Each player shuffles all artifacts they own into their library. Each player who shuffled a nontoken artifact into their library this way reveals cards from the top of their library until they reveal an artifact card, then puts that card onto the battlefield and the rest on the bottom of their library in a random order.",
        ))
        .unwrap();
        assert!(
            shape
                .shuffled_filter
                .card_types
                .contains(&CardType::Artifact)
        );
        assert!(shape.qualifying_filter.nontoken);
        assert!(shape.match_filter.card_types.contains(&CardType::Artifact));
    }
}
