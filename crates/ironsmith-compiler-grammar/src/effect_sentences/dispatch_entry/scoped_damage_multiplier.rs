//! Resolving damage multipliers scoped by a leading duration or to the next
//! damage event, whose source may be a referenced object (CR 614.1a):
//! - "Until your next turn, if that creature would deal combat damage to one
//!   of your opponents, it deals triple that damage to that player instead."
//!   (Jeska, Thrice Reborn)
//! - "until your next turn, if a source would deal damage to that player or a
//!   permanent that player controls, it deals double that damage instead."
//!   (Lightning, Army of One)
//! - "the next time that creature would deal combat damage this turn, it
//!   deals double that damage instead." (Impulsive Maneuvers)
//!
//! The source and recipients the instruction names are locked as it resolves
//! (CR 611.2c); the engine binds them when it registers the replacement.
use super::*;
use crate::grammar::primitives;
use winnow::combinator::{alt, opt};
use winnow::prelude::*;

type Stream<'a> = crate::lexer::LexStream<'a>;
type PResult<T> = Result<T, winnow::error::ErrMode<winnow::error::ContextError>>;

fn leading_duration(input: &mut Stream<'_>) -> PResult<ironsmith_core::ReplacementApplyMode> {
    let mode = alt((
        primitives::phrase(&["until", "end", "of", "turn"])
            .value(ironsmith_core::ReplacementApplyMode::UntilEndOfTurn),
        primitives::phrase(&["until", "your", "next", "turn"])
            .value(ironsmith_core::ReplacementApplyMode::UntilYourNextTurn),
    ))
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    Ok(mode)
}

/// "that creature" / "it": the object the instruction already names.
fn is_referenced_source(tokens: &[OwnedLexToken]) -> bool {
    primitives::probe_all(
        tokens,
        alt((
            primitives::kw("it").void(),
            (
                primitives::kw("that"),
                alt((primitives::kw("creature"), primitives::kw("permanent"))),
            )
                .void(),
        )),
        "referenced damage source",
    )
    .is_some()
}

fn referenced_source_filter() -> ObjectFilter {
    ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind())
}

fn find_word(tokens: &[OwnedLexToken], word: &'static str) -> Option<usize> {
    primitives::find_prefix(tokens, || primitives::kw(word).void()).map(|(index, _, _)| index)
}

/// The "if <source> would deal ..." replacement clause. A referenced source is
/// read as "a source" by the shared multiplier grammar and then narrowed to
/// the referenced object.
fn parse_if_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ironsmith_core::RegisterDamageMultiplierEffect>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("if")) {
        return Ok(None);
    }
    let Some(would) = find_word(tokens, "would") else {
        return Ok(None);
    };
    let referenced = would > 1 && is_referenced_source(&tokens[1..would]);
    let rewritten;
    let clause = if referenced {
        let mut synthetic = crate::lexer::synthetic_word_tokens(&["if", "a", "source"]);
        synthetic.extend_from_slice(&tokens[would..]);
        rewritten = synthetic;
        rewritten.as_slice()
    } else {
        tokens
    };
    let Some(shape) = crate::grammar::keyword_static_lines::parse_damage_multiplier_tokens(clause)
    else {
        return Ok(None);
    };
    if shape.this_turn || shape.condition_tokens.is_some() {
        return Ok(None);
    }
    let Some(mut spec) = crate::keyword_static::damage_multiplier_parts_from_shape(shape)? else {
        return Ok(None);
    };
    if referenced {
        spec.source_filter = referenced_source_filter();
    }
    Ok(Some(spec))
}

/// "the next time that creature would deal [combat] damage this turn, it
/// deals double/triple that damage instead": a one-shot multiplier.
fn parse_next_time(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::RegisterDamageMultiplierEffect> {
    fn read<'a>(input: &mut Stream<'a>) -> PResult<(bool, u32)> {
        primitives::phrase(&["the", "next", "time"]).parse_next(input)?;
        alt((
            primitives::kw("it").void(),
            (
                primitives::kw("that"),
                alt((primitives::kw("creature"), primitives::kw("permanent"))),
            )
                .void(),
        ))
        .parse_next(input)?;
        primitives::phrase(&["would", "deal"]).parse_next(input)?;
        let combat_only = opt(primitives::kw("combat")).parse_next(input)?.is_some();
        primitives::phrase(&["damage", "this", "turn"]).parse_next(input)?;
        opt(primitives::comma()).parse_next(input)?;
        primitives::phrase(&["it", "deals"]).parse_next(input)?;
        let factor = alt((
            primitives::kw("double").value(2u32),
            primitives::kw("triple").value(3u32),
        ))
        .parse_next(input)?;
        primitives::phrase(&["that", "damage", "instead"]).parse_next(input)?;
        primitives::sentence_end().parse_next(input)?;
        Ok((combat_only, factor))
    }
    let (combat_only, factor) = primitives::probe_all(tokens, read, "next-time damage multiplier")?;
    Some(ironsmith_core::RegisterDamageMultiplierEffect {
        source_filter: referenced_source_filter(),
        target_player_filter: Some(PlayerFilter::Any),
        target_object_filter: Some(ObjectFilter::default()),
        factor,
        combat_only,
        noncombat_only: false,
        mode: ironsmith_core::ReplacementApplyMode::OneShot,
    })
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let spec = if let Some((mode, rest)) = primitives::parse_prefix(tokens, leading_duration) {
        parse_if_clause(rest)?.map(|mut spec| {
            spec.mode = mode;
            spec
        })
    } else {
        parse_next_time(tokens)
    };
    Ok(spec.map(|spec| {
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDamageMultiplier {
                spec,
            }),
        )
    }))
}
