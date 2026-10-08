//! "If <creature> attacks, <creatures> attack if able." (Viashino Bey,
//! War's Toll, Magnetic Web): an attack requirement that exists only for an
//! attack declaration in which a creature matching the condition attacks
//! (CR 508.1d). The engine fixes the active set from the proposed declaration.
use crate::cards::builders::{CardTextError, OwnedLexToken};
use crate::grammar::primitives;
use crate::lexer::{token_word_refs, trim_lexed_commas};
use crate::target::{ObjectFilter, PlayerFilter};
use winnow::Parser as _;
use winnow::combinator::alt;

fn object_filter(tokens: &[OwnedLexToken]) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = trim_lexed_commas(tokens);
    if tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = match crate::object_filters::parse_object_filter(tokens, false) {
        Ok(filter) => filter,
        // "creatures with magnet counters on them": the plural pronoun names
        // each matching creature, as "on it" does for one.
        Err(error) => {
            let Some((head, ())) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
                (primitives::phrase(&["on", "them"]), primitives::sentence_end()).void()
            }) else {
                return Err(error);
            };
            crate::object_filters::parse_object_filter(head, false)?
        }
    };
    filter.zone = Some(crate::zone::Zone::Battlefield);
    Ok(Some(filter))
}

pub fn parse_conditional_attack_requirement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::model::CompilerStaticAbilityCore>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(((), after_if)) = primitives::parse_prefix(tokens, primitives::kw("if").void())
    else {
        return Ok(None);
    };
    let Some((attacks, (), rest)) = primitives::find_prefix(after_if, || {
        (primitives::kw("attacks"), primitives::comma()).void()
    }) else {
        return Ok(None);
    };
    let Some((required_tokens, ())) = primitives::split_lexed_once_before_suffix(rest, 1, || {
        (
            primitives::phrase(&["attack", "if", "able"]),
            primitives::sentence_end(),
        )
            .void()
    }) else {
        return Ok(None);
    };
    let trigger_tokens = &after_if[..attacks];
    let trigger = if crate::util::is_source_reference_words(&token_word_refs(trigger_tokens)) {
        ObjectFilter::source()
    } else {
        let Some(((), body)) = primitives::parse_prefix(
            trigger_tokens,
            alt((primitives::kw("a"), primitives::kw("an"))).void(),
        ) else {
            return Ok(None);
        };
        let Some(filter) = object_filter(body)? else {
            return Ok(None);
        };
        filter
    };
    let Some(((), required_body)) =
        primitives::parse_prefix(required_tokens, primitives::kw("all").void())
    else {
        return Ok(None);
    };
    // "all creatures that opponent controls" (War's Toll): the opponent whose
    // creature attacked, which is the attacking (active) player (CR 506.2).
    let required = if let Some((noun, ())) =
        primitives::split_lexed_once_before_suffix(required_body, 1, || {
            primitives::phrase(&["that", "opponent", "controls"])
        }) {
        let Some(mut filter) = object_filter(noun)? else {
            return Ok(None);
        };
        filter.controller = Some(PlayerFilter::Active);
        filter
    } else {
        let Some(filter) = object_filter(required_body)? else {
            return Ok(None);
        };
        filter
    };
    Ok(Some(
        crate::model::CompilerStaticAbilityCore::conditional_attack_requirement(trigger, required),
    ))
}
