use winnow::combinator::{alt, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::runtime_backend::front_end::grammar::{filters, primitives};
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, split_lexed_sentences, trim_lexed_commas,
};
use crate::target::ObjectFilter;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ControllerSacrificeConsultShape {
    pub(crate) target_filter: ObjectFilter,
    pub(crate) match_filter: ObjectFilter,
    pub(crate) destination: Zone,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EachPlayerShuffleThenConsultShape {
    pub(crate) shuffled_filter: ObjectFilter,
    pub(crate) qualifying_filter: ObjectFilter,
    pub(crate) match_filter: ObjectFilter,
    pub(crate) destination: Zone,
    pub(crate) remainder_order: LibraryBottomOrderAst,
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

pub(crate) fn parse_controller_sacrifice_consult_tokens(
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
    let mut target_filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(target_tokens),
        false,
    )
    .ok()?;
    target_filter.zone = Some(Zone::Battlefield);
    let (_, match_tokens) =
        primitives::parse_prefix(trim_lexed_commas(consult_tokens), controller_consult_head)?;
    let mut match_filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(match_tokens),
        false,
    )
    .ok()?;
    match_filter.zone = None;
    let destination = primitives::parse_all_or_none(
        trim_lexed_commas(followup),
        controller_consult_followup,
        "controller sacrifice consult followup",
    )
    .ok()
    .flatten()?;
    Some(ControllerSacrificeConsultShape {
        target_filter,
        match_filter,
        destination,
    })
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

pub(crate) fn parse_each_player_shuffle_then_consult_tokens(
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
    let mut shuffled_filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(shuffled_tokens),
        false,
    )
    .ok()?;
    shuffled_filter.zone = Some(Zone::Battlefield);

    let (_, qualifying_tail) = primitives::parse_prefix(
        consult,
        primitives::phrase(&["each", "player", "who", "shuffled"]).void(),
    )?;
    let (qualifying_tokens, consult_tail) =
        primitives::split_lexed_once_on_separator(qualifying_tail, || qualifying_shuffle_suffix)?;
    let mut qualifying_filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(qualifying_tokens),
        false,
    )
    .ok()?;
    qualifying_filter.zone = Some(Zone::Battlefield);
    let (_, match_and_disposition) =
        primitives::parse_prefix(trim_lexed_commas(consult_tail), each_player_consult_head)?;
    let (match_tokens, disposition_tokens) =
        primitives::split_lexed_once_on_separator(match_and_disposition, || {
            primitives::phrase(&["then", "puts", "that", "card", "onto"]).void()
        })?;
    let mut match_filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(match_tokens),
        false,
    )
    .ok()?;
    match_filter.zone = None;
    let destination = primitives::parse_all_or_none(
        trim_lexed_commas(disposition_tokens),
        random_bottom_disposition,
        "each-player consult disposition",
    )
    .ok()
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
    use crate::runtime_backend::front_end::lexer::lex_line;
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
