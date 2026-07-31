use crate::cards::builders::{IfResultPredicate, OwnedLexToken};
use crate::color::ColorSet;
use crate::filter::CounterConstraint;
use crate::object::CounterType;
use crate::runtime_backend::front_end::grammar::{filters, leaf, primitives};
use crate::runtime_backend::front_end::lexer::{
    LexStream, parser_token_word_refs, trim_lexed_commas,
};
use crate::types::Subtype;
use winnow::Parser as _;
use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TopLibraryAction {
    Exile,
    Reveal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TopLibraryCountShape {
    pub(crate) action: TopLibraryAction,
    pub(crate) count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FutureZoneCounterShape {
    pub(crate) counter_type: CounterType,
    pub(crate) count: u32,
}

pub(crate) fn parse_countered_spell_library_placement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::ZoneReplacementLibraryPlacement> {
    if !marker_present(tokens, primitives::phrase(&["countered", "this", "way"]))
        || !marker_present(tokens, primitives::kw("library"))
    {
        return None;
    }
    if marker_present(
        tokens,
        primitives::phrase(&["choice", "of", "the", "top", "or", "bottom"]),
    ) {
        return Some(ironsmith_core::ZoneReplacementLibraryPlacement::TopOrBottom);
    }
    if marker_present(tokens, primitives::phrase(&["on", "the", "bottom"])) {
        return Some(ironsmith_core::ZoneReplacementLibraryPlacement::Bottom);
    }
    if marker_present(
        tokens,
        alt((
            primitives::phrase(&["on", "top"]),
            primitives::phrase(&["on", "the", "top"]),
        )),
    ) {
        return Some(ironsmith_core::ZoneReplacementLibraryPlacement::Top);
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WhereXReplacementScope {
    DamageOrLife,
    AnyEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WhereXUsageShape<'a> {
    pub(crate) binding_tokens: &'a [OwnedLexToken],
    pub(crate) scope: WhereXReplacementScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaggedCharacteristicsShape<'a> {
    pub(crate) colors: ColorSet,
    pub(crate) subtypes: Vec<Subtype>,
    pub(crate) ability_word: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectAtomicActionShape {
    Learn,
    TimeTravel,
}

fn marker_present<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> bool
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), parser)
        .parse_next(&mut input)
        .is_ok()
}

pub(crate) fn is_that_object_power_damage_to_source_tokens(tokens: &[OwnedLexToken]) -> bool {
    marker_present(
        tokens,
        alt((
            primitives::phrase(&[
                "that", "creature", "deals", "damage", "equal", "to", "its", "power",
            ]),
            primitives::phrase(&[
                "that",
                "permanent",
                "deals",
                "damage",
                "equal",
                "to",
                "its",
                "power",
            ]),
        )),
    ) && marker_present(
        tokens,
        alt((
            primitives::phrase(&["to", "this", "creature"]),
            primitives::phrase(&["to", "this", "permanent"]),
        )),
    )
}

pub(crate) fn has_to_that_player_damage_target_tokens(tokens: &[OwnedLexToken]) -> bool {
    marker_present(tokens, primitives::phrase(&["to", "that", "player"]))
}

fn counter_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("counter"), primitives::kw("counters")))
        .void()
        .parse_next(input)
}

fn counter_holder<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("on").parse_next(input)?;
    alt((primitives::kw("it"), primitives::kw("them")))
        .void()
        .parse_next(input)
}

fn trailing_counter_constraint<'a>(input: &mut LexStream<'a>) -> WResult<CounterConstraint> {
    primitives::kw("with").parse_next(input)?;
    let constraint_tokens = (
        repeat_till(0.., any.void(), peek((counter_noun, counter_holder))).map(|((), _)| ()),
        counter_noun,
        counter_holder,
    )
        .take()
        .parse_next(input)?;
    let refs = parser_token_word_refs(constraint_tokens);
    filters::parse_filter_counter_constraint_words(&refs)
        .map(|(constraint, _)| constraint)
        .ok_or_else(|| {
            primitives::backtrack_err("trailing counter constraint", "counter type on it or them")
        })
}

pub(crate) fn parse_trailing_counter_constraint_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CounterConstraint> {
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), trailing_counter_constraint)
        .parse_next(&mut input)
        .ok()
        .map(|(_, constraint)| constraint)
}

fn top_library_action<'a>(input: &mut LexStream<'a>) -> WResult<TopLibraryAction> {
    alt((
        primitives::kw("exile").value(TopLibraryAction::Exile),
        primitives::kw("reveal").value(TopLibraryAction::Reveal),
    ))
    .parse_next(input)
}

fn parse_top_library_count_lexed<'a>(input: &mut LexStream<'a>) -> WResult<TopLibraryCountShape> {
    let action = top_library_action.parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("top").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    primitives::phrase(&["of", "your", "library"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(TopLibraryCountShape { action, count })
}

pub(crate) fn parse_top_library_count_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TopLibraryCountShape> {
    primitives::parse_all(
        trim_lexed_commas(tokens),
        parse_top_library_count_lexed,
        "top cards of your library",
    )
    .ok()
}

fn future_zone_counter<'a>(input: &mut LexStream<'a>) -> WResult<FutureZoneCounterShape> {
    primitives::kw("with").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    let counter_tokens = (
        repeat_till(
            1..,
            any.void(),
            peek((counter_noun, primitives::phrase(&["on", "it"]))),
        )
        .map(|((), _)| ()),
        counter_noun,
    )
        .take()
        .parse_next(input)?;
    primitives::phrase(&["on", "it"]).parse_next(input)?;
    let counter_type =
        filters::parse_counter_type_from_tokens(counter_tokens).ok_or_else(|| {
            primitives::backtrack_err("future-zone counters", "recognized counter type")
        })?;
    Ok(FutureZoneCounterShape {
        counter_type,
        count,
    })
}

pub(crate) fn parse_future_zone_counter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<FutureZoneCounterShape> {
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), future_zone_counter)
        .parse_next(&mut input)
        .ok()
        .map(|(_, parsed)| parsed)
}

fn where_x_split<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    let leading = repeat_till(
        0..,
        any.void(),
        peek(primitives::phrase(&["where", "x", "is"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    let binding = (
        primitives::phrase(&["where", "x", "is"]),
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::sentence_end())),
        primitives::sentence_end(),
    )
        .take()
        .parse_next(input)?;
    Ok((leading, binding))
}

pub(crate) fn parse_where_x_usage_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<WhereXUsageShape<'_>> {
    let (leading, full_binding_tokens) =
        primitives::parse_all(tokens, where_x_split, "where X binding").ok()?;
    let binding_tokens = full_binding_tokens
        .windows(2)
        .position(|window| window[0].is_comma() && window[1].is_word("then"))
        .map_or(full_binding_tokens, |split| &full_binding_tokens[..split]);
    let damage_or_life = marker_present(
        leading,
        alt((
            primitives::phrase(&["deal", "x", "damage"]),
            primitives::phrase(&["deals", "x", "damage"]),
            primitives::phrase(&["gain", "x", "life"]),
            primitives::phrase(&["gains", "x", "life"]),
            primitives::phrase(&["lose", "x", "life"]),
            primitives::phrase(&["loses", "x", "life"]),
        )),
    );
    let scope = if damage_or_life {
        WhereXReplacementScope::DamageOrLife
    } else if marker_present(leading, primitives::kw("x")) {
        WhereXReplacementScope::AnyEffect
    } else {
        return None;
    };
    Some(WhereXUsageShape {
        binding_tokens,
        scope,
    })
}

fn parse_flip_result_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(IfResultPredicate, &'a [OwnedLexToken])> {
    primitives::kw("if").parse_next(input)?;
    primitives::kw("you").parse_next(input)?;
    let predicate = alt((
        primitives::kw("win").value(IfResultPredicate::Did),
        primitives::kw("lose").value(IfResultPredicate::DidNot),
    ))
    .parse_next(input)?;
    primitives::phrase(&["the", "flip"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let effects = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok((predicate, trim_lexed_commas(effects)))
}

pub(crate) fn parse_flip_result_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(IfResultPredicate, &[OwnedLexToken])> {
    primitives::parse_all(tokens, parse_flip_result_lexed, "flip result sentence").ok()
}

fn tagged_characteristics_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("they're").void(),
        primitives::kw("they’re").void(),
        primitives::kw("theyre").void(),
        primitives::phrase(&["they", "are"]),
    ))
    .parse_next(input)
}

fn descriptor_before_addition(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut input = LexStream::new(tokens);
    if let Ok((leading, _)) = repeat_till::<_, _, Vec<&OwnedLexToken>, _, _, _, _>(
        0..,
        any,
        primitives::phrase(&["in", "addition", "to"]),
    )
    .parse_next(&mut input)
    {
        let count = leading.len();
        &tokens[..count]
    } else {
        tokens
    }
}

fn parse_tagged_characteristics_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TaggedCharacteristicsShape<'a>> {
    tagged_characteristics_prefix.parse_next(input)?;
    let raw_descriptor = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["and", "they", "gain"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["and", "they", "gain"]).parse_next(input)?;
    let ability = primitives::word_parser_text.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let descriptor = trim_lexed_commas(descriptor_before_addition(raw_descriptor));
    let mut colors = ColorSet::new();
    let mut subtypes = Vec::new();
    for token in descriptor {
        let Some(word) = token.as_word() else {
            continue;
        };
        if primitives::parse_all(
            std::slice::from_ref(token),
            alt((primitives::kw("and"), primitives::kw("or"))).void(),
            "characteristic connector",
        )
        .is_ok()
        {
            continue;
        }
        if let Ok(color) = leaf::parse_leaf_color_complete(word) {
            colors = colors.union(color);
            continue;
        }
        let subtype = leaf::parse_leaf_subtype_flexible_complete(word).map_err(|_| {
            primitives::backtrack_err("tagged characteristics", "color or creature type")
        })?;
        if subtypes.iter().all(|existing| existing != &subtype) {
            subtypes.push(subtype);
        }
    }
    if colors.is_empty() && subtypes.is_empty() {
        return Err(primitives::backtrack_err(
            "tagged characteristics",
            "at least one color or subtype",
        ));
    }
    Ok(TaggedCharacteristicsShape {
        colors,
        subtypes,
        ability_word: ability,
    })
}

pub(crate) fn parse_tagged_characteristics_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TaggedCharacteristicsShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_tagged_characteristics_lexed,
        "tagged characteristics and keyword",
    )
    .ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutsideGameArtRatingSentenceKind {
    Request,
    ResultTrigger,
}

pub(crate) fn parse_outside_game_art_rating_tokens(
    tokens: &[OwnedLexToken],
) -> Option<OutsideGameArtRatingSentenceKind> {
    if marker_present(
        tokens,
        primitives::phrase(&["ask", "a", "person", "outside", "the", "game", "to", "rate"]),
    ) {
        return Some(OutsideGameArtRatingSentenceKind::Request);
    }
    marker_present(
        tokens,
        primitives::phrase(&["when", "they", "rate", "the", "art"]),
    )
    .then_some(OutsideGameArtRatingSentenceKind::ResultTrigger)
}

pub(crate) fn is_outside_game_art_rating_tokens(tokens: &[OwnedLexToken]) -> bool {
    parse_outside_game_art_rating_tokens(tokens).is_some()
}

pub(crate) fn is_one_or_more_this_way_tokens(tokens: &[OwnedLexToken]) -> bool {
    marker_present(tokens, primitives::phrase(&["one", "or", "more"]))
        && marker_present(tokens, primitives::phrase(&["this", "way"]))
}

fn direct_for_each_who<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(primitives::kw("for")).parse_next(input)?;
    primitives::kw("each").parse_next(input)?;
    alt((
        primitives::kw("opponent"),
        primitives::kw("opponents"),
        primitives::kw("player"),
        primitives::kw("players"),
    ))
    .parse_next(input)?;
    primitives::kw("who").void().parse_next(input)
}

pub(crate) fn is_direct_for_each_who_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, direct_for_each_who).is_some()
}

fn direct_atomic_action<'a>(input: &mut LexStream<'a>) -> WResult<DirectAtomicActionShape> {
    alt((
        (primitives::kw("learn"), primitives::sentence_end()).value(DirectAtomicActionShape::Learn),
        (
            primitives::phrase(&["time", "travel"]),
            primitives::sentence_end(),
        )
            .value(DirectAtomicActionShape::TimeTravel),
    ))
    .parse_next(input)
}

pub(crate) fn parse_direct_atomic_action_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DirectAtomicActionShape> {
    primitives::parse_all(tokens, direct_atomic_action, "direct atomic action").ok()
}

fn otherwise_referential_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("that").parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("permanent"))).parse_next(input)?;
    alt((
        primitives::kw("get"),
        primitives::kw("gets"),
        primitives::kw("gain"),
        primitives::kw("gains"),
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn has_otherwise_referential_subject_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, otherwise_referential_subject).is_some()
}

fn x_cant_be_zero<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("x").parse_next(input)?;
    alt((primitives::kw("cant"), primitives::kw("can't"))).parse_next(input)?;
    primitives::phrase(&["be", "0"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

pub(crate) fn is_x_cant_be_zero_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(tokens, x_cant_be_zero, "X cannot be zero").is_ok()
}

fn token_granted_ability<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    alt((
        alt((
            primitives::phrase(&["it", "has"]),
            primitives::phrase(&["it", "gains"]),
            primitives::phrase(&["that", "token", "has"]),
            primitives::phrase(&["that", "token", "gains"]),
            primitives::phrase(&["the", "token", "has"]),
            primitives::phrase(&["the", "token", "gains"]),
        )),
        alt((
            primitives::phrase(&["they", "have"]),
            primitives::phrase(&["they", "gain"]),
            primitives::phrase(&["those", "tokens", "have"]),
            primitives::phrase(&["those", "tokens", "gain"]),
            primitives::phrase(&["the", "tokens", "have"]),
            primitives::phrase(&["the", "tokens", "gain"]),
        )),
    ))
    .parse_next(input)?;
    let abilities = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(trim_lexed_commas(abilities))
}

pub(crate) fn parse_token_granted_ability_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    primitives::parse_all(tokens, token_granted_ability, "token granted ability").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_counter_and_top_library_facts() {
        let constraint = lex_line(
            "Destroy all creatures with two or more stun counters on them.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_trailing_counter_constraint_tokens(&constraint),
            Some(CounterConstraint::AtLeast {
                counter_type: Some(CounterType::Stun),
                count: 2,
            })
        );

        let reveal = lex_line("Reveal the top three cards of your library.", 0).unwrap();
        assert_eq!(
            parse_top_library_count_tokens(&reveal),
            Some(TopLibraryCountShape {
                action: TopLibraryAction::Reveal,
                count: 3,
            })
        );

        let top = lex_line(
            "If that spell is countered this way, put it on top of its owner's library instead.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_countered_spell_library_placement_tokens(&top),
            Some(ironsmith_core::ZoneReplacementLibraryPlacement::Top)
        );
        let bottom = lex_line(
            "If that spell is countered this way, put it on the bottom of its owner's library instead.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_countered_spell_library_placement_tokens(&bottom),
            Some(ironsmith_core::ZoneReplacementLibraryPlacement::Bottom)
        );
        let choice = lex_line(
            "If that spell is countered this way, put that card on your choice of the top or bottom of its owner's library instead.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_countered_spell_library_placement_tokens(&choice),
            Some(ironsmith_core::ZoneReplacementLibraryPlacement::TopOrBottom)
        );
    }

    #[test]
    fn parses_flip_characteristic_and_ability_prefixes() {
        let flip = lex_line("If you win the flip, draw two cards.", 0).unwrap();
        assert!(matches!(
            parse_flip_result_shape_tokens(&flip),
            Some((IfResultPredicate::Did, _))
        ));

        let tagged = lex_line("They are black Zombies and they gain flying.", 0).unwrap();
        let parsed = parse_tagged_characteristics_shape_tokens(&tagged).unwrap();
        assert_eq!(parsed.ability_word, "flying");
        assert_eq!(parsed.subtypes.len(), 1);

        let ability = lex_line("Those tokens gain haste.", 0).unwrap();
        assert!(parse_token_granted_ability_tokens(&ability).is_some());
    }

    #[test]
    fn where_x_binding_stops_before_a_trailing_then_action() {
        let tokens = lex_line(
            "This artifact deals X damage to any target, where X is the total power of the \
             creatures sacrificed this way, then exile this artifact.",
            0,
        )
        .unwrap();
        let shape = parse_where_x_usage_shape_tokens(&tokens).expect("where-X binding");

        assert_eq!(
            parser_token_word_refs(shape.binding_tokens),
            [
                "where",
                "x",
                "is",
                "the",
                "total",
                "power",
                "of",
                "the",
                "creatures",
                "sacrificed",
                "this",
                "way"
            ]
        );
    }
}
