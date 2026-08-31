use winnow::combinator::{alt, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::model::token_definition::{
    CreatureTokenRulesShape, CreatureTokenShape, TokenDefinitionSpec,
};
use crate::object::CounterType;
use ironsmith_core::DamagedBySource;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::super::{filters, leaf, primitives, token_definitions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementPlayerKind {
    Any,
    You,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleSourceReplacementKind {
    Any,
    Creature,
    Artifact,
    Enchantment,
    Permanent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExileWouldDieVictimKind {
    Creature,
    Permanent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExileGraveyardFilterKind {
    Source,
    AnyCard,
    CreatureCard,
    CyclingCard,
    ObjectFilter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExileToGraveyardReplacementSpec<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub filter_kind: ExileGraveyardFilterKind,
    pub graveyard_owner: ReplacementPlayerKind,
    pub exclude_cycled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExileWouldDieSpec {
    NontokenCreature {
        controller: ReplacementPlayerKind,
        exile_counter: Option<CounterType>,
        follow_up_token: Option<CreatureTokenShape>,
    },
    DamagedBy {
        victim: ExileWouldDieVictimKind,
        damaged_by: DamagedBySource,
    },
    DamagedByFilter {
        victim: ExileWouldDieVictimKind,
        damager_filter_tokens: Vec<OwnedLexToken>,
    },
    SimpleSource(SimpleSourceReplacementKind),
    SimpleCreature(ReplacementPlayerKind),
}

pub fn parse_exile_to_graveyard_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ExileToGraveyardReplacementSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_exile_to_graveyard_replacement_lexed,
        "exile instead of graveyard replacement",
    )
}

pub fn parse_exile_would_die_tokens(tokens: &[OwnedLexToken]) -> Option<ExileWouldDieSpec> {
    crate::grammar::primitives::probe_all(
        tokens,
        alt((
            parse_nontoken_exile_would_die_lexed,
            parse_damaged_by_exile_would_die_lexed,
            parse_simple_source_exile_would_die_lexed,
            parse_simple_creature_exile_would_die_lexed,
        )),
        "exile would-die replacement",
    )
}

pub fn parse_you_controlled_source_filter_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            opt(primitives::kw("a")),
            primitives::kw("source"),
            primitives::kw("you"),
            alt((primitives::kw("control"), primitives::kw("controlled"))),
            primitives::sentence_end(),
        )
            .void(),
        "source controlled by you",
    )
    .is_ok()
}

fn parse_exile_to_graveyard_replacement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExileToGraveyardReplacementSpec<'a>> {
    primitives::kw("if").parse_next(input)?;
    let filter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["would", "be", "put", "into"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["would", "be", "put", "into"]).parse_next(input)?;
    let graveyard_owner = parse_graveyard_owner_lexed(input)?;
    primitives::kw("graveyard").parse_next(input)?;
    primitives::phrase(&["from", "anywhere"]).parse_next(input)?;
    let exclude_cycled = opt((
        primitives::kw("and"),
        primitives::kw("it"),
        alt((primitives::kw("wasnt"), primitives::kw("wasn't"))),
        primitives::kw("cycled"),
    ))
    .map(|clause| clause.is_some())
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("exile").parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        primitives::phrase(&["that", "card"]),
    ))
    .parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let filter_tokens = trim_lexed_commas(filter_tokens);
    let filter_kind = classify_exile_graveyard_filter(filter_tokens);
    Ok(ExileToGraveyardReplacementSpec {
        filter_tokens,
        filter_kind,
        graveyard_owner,
        exclude_cycled,
    })
}

fn classify_exile_graveyard_filter(tokens: &[OwnedLexToken]) -> ExileGraveyardFilterKind {
    let words = TokenWordView::new(tokens).word_refs();
    if leaf::parse_leaf_this_source_reference_words(&words).is_some()
        || crate::util::source_reference_surface_for_words(&words).is_some()
    {
        return ExileGraveyardFilterKind::Source;
    }
    if exact_phrase(tokens, &["a", "card", "or", "token"])
        || exact_phrase(tokens, &["card", "or", "token"])
        || exact_phrase(tokens, &["a", "card"])
        || exact_phrase(tokens, &["card"])
    {
        return ExileGraveyardFilterKind::AnyCard;
    }
    if exact_phrase(tokens, &["a", "creature", "card"])
        || exact_phrase(tokens, &["creature", "card"])
    {
        return ExileGraveyardFilterKind::CreatureCard;
    }
    if exact_phrase(
        tokens,
        &["a", "card", "that", "has", "a", "cycling", "ability"],
    ) || exact_phrase(tokens, &["card", "that", "has", "a", "cycling", "ability"])
    {
        return ExileGraveyardFilterKind::CyclingCard;
    }
    ExileGraveyardFilterKind::ObjectFilter
}

fn parse_graveyard_owner_lexed(input: &mut LexStream<'_>) -> WResult<ReplacementPlayerKind> {
    alt((
        primitives::kw("your").value(ReplacementPlayerKind::You),
        alt((
            primitives::phrase(&["an", "opponents"]),
            primitives::phrase(&["an", "opponent's"]),
            primitives::kw("opponents").void(),
            primitives::kw("opponent's").void(),
        ))
        .value(ReplacementPlayerKind::Opponent),
        alt((
            primitives::phrase(&["a", "players"]),
            primitives::phrase(&["a", "player's"]),
            primitives::kw("a").void(),
        ))
        .value(ReplacementPlayerKind::Any),
    ))
    .parse_next(input)
}

fn parse_nontoken_exile_would_die_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExileWouldDieSpec> {
    primitives::kw("if").parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    primitives::phrase(&["nontoken", "creature"]).parse_next(input)?;
    let controller = opt(alt((
        alt((
            primitives::phrase(&["an", "opponent", "controls"]),
            primitives::phrase(&["opponent", "controls"]),
        ))
        .value(ReplacementPlayerKind::Opponent),
        primitives::phrase(&["you", "control"]).value(ReplacementPlayerKind::You),
    )))
    .map(|controller| controller.unwrap_or(ReplacementPlayerKind::Any))
    .parse_next(input)?;
    primitives::phrase(&["would", "die"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let (exile_counter, follow_up_token) = alt((
        parse_created_token_exile_tail_lexed.map(|token| (None, Some(token))),
        parse_countered_exile_tail_lexed.map(|counter| (Some(counter), None)),
        parse_plain_exile_tail_lexed.value((None, None)),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ExileWouldDieSpec::NontokenCreature {
        controller,
        exile_counter,
        follow_up_token,
    })
}

fn parse_created_token_exile_tail_lexed(input: &mut LexStream<'_>) -> WResult<CreatureTokenShape> {
    alt((
        (
            primitives::kw("exile"),
            primitives::phrase(&["that", "card", "instead", "and", "create"]),
        ),
        (
            primitives::kw("instead"),
            primitives::phrase(&["exile", "that", "card", "and", "create"]),
        ),
    ))
    .parse_next(input)?;
    let definition_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), ())| ())
            .take()
            .parse_next(input)?;
    match token_definitions::parse_token_definition_shape_tokens(definition_tokens) {
        Some(TokenDefinitionSpec::Creature(shape))
            if shape.rules == CreatureTokenRulesShape::default() =>
        {
            Ok(shape)
        }
        _ => Err(primitives::backtrack_err(
            "would-die replacement follow-up",
            "creature token definition",
        )),
    }
}

fn parse_countered_exile_tail_lexed(input: &mut LexStream<'_>) -> WResult<CounterType> {
    let leading_instead = opt(primitives::kw("instead"))
        .map(|instead| instead.is_some())
        .parse_next(input)?;
    primitives::kw("exile").parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        primitives::phrase(&["that", "card"]),
    ))
    .parse_next(input)?;
    primitives::kw("with").parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    let counter_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("counter")))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::phrase(&["counter", "on", "it"]).parse_next(input)?;
    if !leading_instead {
        primitives::kw("instead").parse_next(input)?;
    }
    filters::parse_counter_type_from_tokens(trim_lexed_commas(counter_tokens)).ok_or_else(|| {
        primitives::backtrack_err(
            "countered exile would-die replacement",
            "known counter type",
        )
    })
}

fn parse_plain_exile_tail_lexed(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        (
            primitives::kw("exile"),
            alt((
                primitives::kw("it").void(),
                primitives::phrase(&["that", "card"]),
            )),
            primitives::kw("instead"),
        )
            .void(),
        (
            primitives::kw("instead"),
            primitives::kw("exile"),
            alt((
                primitives::kw("it").void(),
                primitives::phrase(&["that", "card"]),
            )),
        )
            .void(),
    ))
    .parse_next(input)
}

fn parse_damaged_by_exile_would_die_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExileWouldDieSpec> {
    primitives::kw("if").parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    let victim = alt((
        primitives::kw("creature").value(ExileWouldDieVictimKind::Creature),
        primitives::kw("permanent").value(ExileWouldDieVictimKind::Permanent),
    ))
    .parse_next(input)?;
    primitives::phrase(&["dealt", "damage"]).parse_next(input)?;
    let source_tokens = if primitives::phrase(&["this", "turn", "by"])
        .parse_next(&mut input.clone())
        .is_ok()
    {
        primitives::phrase(&["this", "turn", "by"]).parse_next(input)?;
        let source_tokens = repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek(primitives::phrase(&["would", "die"])),
        )
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
        primitives::phrase(&["would", "die"]).parse_next(input)?;
        source_tokens
    } else {
        primitives::kw("by").parse_next(input)?;
        let source_tokens = repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek(primitives::phrase(&["this", "turn", "would", "die"])),
        )
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
        primitives::phrase(&["this", "turn", "would", "die"]).parse_next(input)?;
        source_tokens
    };
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["exile", "it", "instead"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let source_tokens = trim_lexed_commas(source_tokens);
    Ok(match classify_damage_source(source_tokens) {
        Ok(damaged_by) => ExileWouldDieSpec::DamagedBy { victim, damaged_by },
        Err(_) => ExileWouldDieSpec::DamagedByFilter {
            victim,
            damager_filter_tokens: source_tokens.to_vec(),
        },
    })
}

fn classify_damage_source(tokens: &[OwnedLexToken]) -> WResult<DamagedBySource> {
    if exact_phrase(tokens, &["equipped", "creature"]) {
        return Ok(DamagedBySource::EquippedCreature);
    }
    if exact_phrase(tokens, &["enchanted", "creature"]) {
        return Ok(DamagedBySource::EnchantedCreature);
    }
    if exact_phrase(tokens, &["this"])
        || exact_phrase(tokens, &["this", "creature"])
        || exact_phrase(tokens, &["this", "permanent"])
        || exact_phrase(tokens, &["this", "source"])
        || is_named_source_reference(tokens)
    {
        return Ok(DamagedBySource::ThisCreature);
    }
    Err(primitives::backtrack_err(
        "damage-source replacement reference",
        "this, attached, or named source",
    ))
}

fn exact_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::parse_all(
        tokens,
        primitives::phrase(phrase),
        "exact replacement phrase",
    )
    .is_ok()
}

fn is_named_source_reference(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    if leaf::parse_leaf_this_source_reference_words(&words).is_some()
        || crate::util::source_reference_surface_for_words(&words).is_some()
    {
        return true;
    }
    primitives::parse_all(
        tokens,
        repeat::<_, _, (), _, _>(
            1..,
            any.verify(|token: &&OwnedLexToken| {
                token.as_word().is_some_and(|word| {
                    !matches!(
                        word,
                        "a" | "an"
                            | "the"
                            | "target"
                            | "that"
                            | "this"
                            | "equipped"
                            | "enchanted"
                            | "creature"
                            | "creatures"
                            | "permanent"
                            | "permanents"
                            | "source"
                            | "sources"
                    )
                })
            })
            .void(),
        ),
        "named damage source",
    )
    .is_ok()
}

fn parse_simple_source_exile_would_die_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExileWouldDieSpec> {
    primitives::phrase(&["if", "this"]).parse_next(input)?;
    let kind = opt(alt((
        primitives::kw("creature").value(SimpleSourceReplacementKind::Creature),
        primitives::kw("artifact").value(SimpleSourceReplacementKind::Artifact),
        primitives::kw("enchantment").value(SimpleSourceReplacementKind::Enchantment),
        primitives::kw("permanent").value(SimpleSourceReplacementKind::Permanent),
        primitives::kw("object").value(SimpleSourceReplacementKind::Any),
    )))
    .map(|kind| kind.unwrap_or(SimpleSourceReplacementKind::Any))
    .parse_next(input)?;
    primitives::phrase(&["would", "die"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["exile", "it", "instead"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ExileWouldDieSpec::SimpleSource(kind))
}

fn parse_simple_creature_exile_would_die_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExileWouldDieSpec> {
    primitives::kw("if").parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    primitives::kw("creature").parse_next(input)?;
    let player = opt(alt((
        alt((
            primitives::phrase(&["an", "opponent", "controls"]),
            primitives::phrase(&["opponent", "controls"]),
        ))
        .value(ReplacementPlayerKind::Opponent),
        primitives::phrase(&["you", "control"]).value(ReplacementPlayerKind::You),
    )))
    .map(|player| player.unwrap_or(ReplacementPlayerKind::Any))
    .parse_next(input)?;
    primitives::phrase(&["would", "die"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["exile", "it", "instead"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ExileWouldDieSpec::SimpleCreature(player))
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_graveyard_and_would_die_replacement_shapes() {
        let tokens = lex_line(
            "If a card that has a cycling ability would be put into your graveyard from anywhere and it wasn't cycled, exile it instead.",
            0,
        )
        .unwrap();
        let spec = parse_exile_to_graveyard_replacement_tokens(&tokens).unwrap();
        assert_eq!(spec.graveyard_owner, ReplacementPlayerKind::You);
        assert!(spec.exclude_cycled);

        let tokens = lex_line(
            "If a nontoken creature an opponent controls would die, exile that card with an ice counter on it instead.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_exile_would_die_tokens(&tokens),
            Some(ExileWouldDieSpec::NontokenCreature {
                controller: ReplacementPlayerKind::Opponent,
                exile_counter: Some(CounterType::Ice),
                ..
            })
        ));

        let tokens = lex_line(
            "If a nontoken creature an opponent controls would die, instead exile that card and create a 2/2 black Zombie creature token.",
            0,
        )
        .unwrap();
        let Some(ExileWouldDieSpec::NontokenCreature {
            follow_up_token: Some(token),
            ..
        }) = parse_exile_would_die_tokens(&tokens)
        else {
            panic!("expected a typed creature-token follow-up")
        };
        assert_eq!(token.power_toughness, (2, 2));
        assert_eq!(token.colors, crate::color::ColorSet::BLACK);
        assert_eq!(token.subtypes, vec![crate::types::Subtype::Zombie]);

        let tokens = lex_line("If this creature would die, exile it instead.", 0).unwrap();
        assert_eq!(
            parse_exile_would_die_tokens(&tokens),
            Some(ExileWouldDieSpec::SimpleSource(
                SimpleSourceReplacementKind::Creature
            ))
        );

        let tokens = lex_line(
            "If a creature dealt damage this turn by a source you controlled would die, exile it instead.",
            0,
        )
        .unwrap();
        let Some(ExileWouldDieSpec::DamagedByFilter {
            victim: ExileWouldDieVictimKind::Creature,
            damager_filter_tokens,
        }) = parse_exile_would_die_tokens(&tokens)
        else {
            panic!("expected a typed filtered-damager replacement")
        };
        assert_eq!(
            TokenWordView::new(&damager_filter_tokens).word_refs(),
            ["a", "source", "you", "controlled"]
        );
    }
}
