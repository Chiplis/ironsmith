use super::*;

use crate::grammar::{filters, leaf};
use crate::object::CounterType;
use crate::types::CardType;
use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till, separated};
use winnow::error::ModalResult as WResult;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterDescriptorShape {
    pub count: u32,
    pub counter_type: CounterType,
    pub additional: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CounterMarkerTimingShape {
    NextEndStep(PlayerFilter),
    NextUpkeep(PlayerAst),
    EndOfCombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterMarkerDestinationShape {
    pub tapped: bool,
    pub attacking: bool,
    pub transformed: bool,
    pub controller: ReturnControllerAst,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveWithCountersShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub destination: CounterMarkerDestinationShape,
    pub descriptors: Vec<CounterDescriptorShape>,
    pub timing: Option<CounterMarkerTimingShape>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SacrificeAtEndOfCombatShape<'a> {
    pub object_tokens: &'a [OwnedLexToken],
    pub tagged_object: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForEachCounterKindShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetsThenFightsShape<'a> {
    pub pump_tokens: &'a [OwnedLexToken],
    pub first_target_tokens: &'a [OwnedLexToken],
    pub second_target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawThenConniveShape<'a> {
    pub draw_tokens: &'a [OwnedLexToken],
    pub connive_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdditionalCounterShape {
    pub descriptor: CounterDescriptorShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalEntryCounterArmShape {
    pub descriptor: CounterDescriptorShape,
    pub object_type: CardType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedConditionalEntryCountersShape {
    pub arms: Vec<ConditionalEntryCounterArmShape>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PutWithAdditionalCounterShape<'a> {
    pub move_tokens: &'a [OwnedLexToken],
    pub descriptor: CounterDescriptorShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SacrificeThenPutAdditionalShape<'a> {
    pub sacrifice_tokens: &'a [OwnedLexToken],
    pub plain_word_sacrifice: bool,
    pub put: PutWithAdditionalCounterShape<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IfSacrificeThenPutAdditionalShape<'a> {
    pub predicate_tokens: &'a [OwnedLexToken],
    pub effect: SacrificeThenPutAdditionalShape<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EachPlayerReturnAdditionalShape<'a> {
    pub return_tokens: &'a [OwnedLexToken],
    pub descriptor: CounterDescriptorShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutCounterChoiceShape<'a> {
    pub counter_types: Vec<CounterType>,
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutFixedAndCounterChoiceShape<'a> {
    pub fixed: CounterDescriptorShape,
    pub counter_types: Vec<CounterType>,
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PutCounterSequenceShape<'a> {
    Plain,
    Then {
        head_tokens: &'a [OwnedLexToken],
        tail_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterPlacementShape<'a> {
    pub descriptor: CounterDescriptorShape,
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedCounterTargetShape<'a> {
    pub descriptors: Vec<CounterDescriptorShape>,
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterFollowupShape<'a> {
    pub counter_tokens: &'a [OwnedLexToken],
    pub followup_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterPairShape<'a> {
    pub first_tokens: &'a [OwnedLexToken],
    pub second_tokens: &'a [OwnedLexToken],
}

fn counter_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("counter"), primitives::kw("counters")))
        .void()
        .parse_next(input)
}

fn amount<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    alt((
        leaf::parse_leaf_number_prefix_lexed,
        alt((primitives::kw("a"), primitives::kw("an"))).value(1),
    ))
    .parse_next(input)
}

fn parse_counter_descriptor_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterDescriptorShape> {
    let count = amount.parse_next(input)?;
    let additional = opt(primitives::kw("additional"))
        .parse_next(input)?
        .is_some();
    let counter_type_tokens = (
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(counter_noun)).void(),
        counter_noun,
    )
        .take()
        .parse_next(input)?;
    let counter_type =
        filters::parse_counter_type_from_tokens(counter_type_tokens).ok_or_else(|| {
            primitives::backtrack_err("counter descriptor", "recognized counter type")
        })?;
    Ok(CounterDescriptorShape {
        count,
        counter_type,
        additional,
    })
}

fn controller_tail<'a>(input: &mut LexStream<'a>) -> WResult<ReturnControllerAst> {
    opt((
        primitives::kw("under"),
        alt((
            owner_reference.value(ReturnControllerAst::Owner),
            alt((primitives::kw("its"), primitives::kw("their")))
                .value(ReturnControllerAst::Preserve),
            primitives::kw("your").value(ReturnControllerAst::You),
        )),
        primitives::kw("control"),
    ))
    .map(|tail| {
        tail.map(|(_, controller, _)| controller)
            .unwrap_or(ReturnControllerAst::Preserve)
    })
    .parse_next(input)
}

fn owner_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        (
            alt((
                primitives::kw("its"),
                primitives::kw("their"),
                primitives::kw("his"),
                primitives::kw("her"),
            )),
            alt((
                primitives::kw("owner"),
                primitives::kw("owners"),
                primitives::kw("owner's"),
                primitives::kw("owner’s"),
            )),
        )
            .void(),
        primitives::phrase(&["that", "player"]),
    ))
    .parse_next(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestinationFlag {
    Tapped,
    Attacking,
    Transformed,
}

fn destination_start<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (opt(primitives::kw("the")), primitives::kw("battlefield"))
        .void()
        .parse_next(input)
}

fn parse_destination_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterMarkerDestinationShape> {
    destination_start.parse_next(input)?;
    let flags: Vec<DestinationFlag> = repeat(
        0..,
        alt((
            primitives::phrase(&["and", "attacking"]).value(DestinationFlag::Attacking),
            primitives::kw("tapped").value(DestinationFlag::Tapped),
            primitives::kw("attacking").value(DestinationFlag::Attacking),
            primitives::kw("transformed").value(DestinationFlag::Transformed),
        )),
    )
    .parse_next(input)?;
    let controller = controller_tail.parse_next(input)?;
    let (tapped, attacking, transformed) = flags.into_iter().fold(
        (false, false, false),
        |(tapped, attacking, transformed), flag| match flag {
            DestinationFlag::Tapped => (true, attacking, transformed),
            DestinationFlag::Attacking => (tapped, true, transformed),
            DestinationFlag::Transformed => (tapped, attacking, true),
        },
    );
    Ok(CounterMarkerDestinationShape {
        tapped,
        attacking,
        transformed,
        controller,
    })
}

fn timing<'a>(input: &mut LexStream<'a>) -> WResult<CounterMarkerTimingShape> {
    alt((
        alt((
            primitives::phrase(&["at", "end", "of", "combat"]),
            primitives::phrase(&["at", "the", "end", "of", "combat"]),
        ))
        .value(CounterMarkerTimingShape::EndOfCombat),
        alt((
            primitives::phrase(&["at", "beginning", "of", "next", "end", "step"]),
            primitives::phrase(&["at", "beginning", "of", "the", "next", "end", "step"]),
            primitives::phrase(&["at", "the", "beginning", "of", "next", "end", "step"]),
            primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "end", "step"]),
        ))
        .value(CounterMarkerTimingShape::NextEndStep(PlayerFilter::Any)),
        alt((
            primitives::phrase(&["at", "beginning", "of", "your", "next", "end", "step"]),
            primitives::phrase(&[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "end",
                "step",
            ]),
        ))
        .value(CounterMarkerTimingShape::NextEndStep(PlayerFilter::You)),
        alt((
            primitives::phrase(&["at", "beginning", "of", "next", "upkeep"]),
            primitives::phrase(&["at", "beginning", "of", "the", "next", "upkeep"]),
            primitives::phrase(&["at", "the", "beginning", "of", "next", "upkeep"]),
            primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "upkeep"]),
        ))
        .value(CounterMarkerTimingShape::NextUpkeep(PlayerAst::Any)),
        alt((
            primitives::phrase(&["at", "beginning", "of", "your", "next", "upkeep"]),
            primitives::phrase(&["at", "the", "beginning", "of", "your", "next", "upkeep"]),
        ))
        .value(CounterMarkerTimingShape::NextUpkeep(PlayerAst::You)),
    ))
    .parse_next(input)
}

fn tagged_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("it"),
        primitives::kw("them"),
        primitives::kw("him"),
        primitives::kw("her"),
    ))
    .void()
    .parse_next(input)
}

fn parse_return_with_counters_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<MoveWithCountersShape<'a>> {
    opt(primitives::kw("then")).parse_next(input)?;
    primitives::kw("return").parse_next(input)?;
    let target_tokens = repeat_till(
        1..,
        any.void(),
        peek((primitives::kw("to"), destination_start)),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::kw("to").parse_next(input)?;
    let destination = parse_destination_lexed.parse_next(input)?;
    primitives::kw("with").parse_next(input)?;
    let descriptors =
        separated(1.., parse_counter_descriptor_lexed, primitives::kw("and")).parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    tagged_reference.parse_next(input)?;
    let timing = opt(timing).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(MoveWithCountersShape {
        target_tokens,
        destination,
        descriptors,
        timing,
    })
}

pub fn parse_return_with_counters_tokens(
    tokens: &[OwnedLexToken],
) -> Option<MoveWithCountersShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_return_with_counters_lexed,
        "return with counters",
    )
    .ok()
}

fn parse_put_onto_battlefield_with_counters_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<MoveWithCountersShape<'a>> {
    alt((primitives::kw("put"), primitives::kw("puts"))).parse_next(input)?;
    let target_tokens = repeat_till(1.., any.void(), peek(primitives::kw("onto")))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::kw("onto").parse_next(input)?;
    let destination = parse_destination_lexed.parse_next(input)?;
    primitives::kw("with").parse_next(input)?;
    let descriptors =
        separated(1.., parse_counter_descriptor_lexed, primitives::kw("and")).parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    tagged_reference.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(MoveWithCountersShape {
        target_tokens,
        destination,
        descriptors,
        timing: None,
    })
}

pub fn parse_put_onto_battlefield_with_counters_tokens(
    tokens: &[OwnedLexToken],
) -> Option<MoveWithCountersShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_put_onto_battlefield_with_counters_lexed,
        "put onto battlefield with counters",
    )
    .ok()
}

fn end_of_combat<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["at", "end", "of", "combat"]),
        primitives::phrase(&["at", "the", "end", "of", "combat"]),
    ))
    .parse_next(input)
}

fn parse_sacrifice_at_end_of_combat_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SacrificeAtEndOfCombatShape<'a>> {
    primitives::kw("sacrifice").parse_next(input)?;
    let object_tokens = repeat_till(1.., any.void(), peek(end_of_combat))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    end_of_combat.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let tagged_object = primitives::parse_all(
        object_tokens,
        (
            alt((
                primitives::kw("it").void(),
                primitives::kw("them").void(),
                primitives::phrase(&["that", "token"]),
                primitives::phrase(&["this", "token"]),
                primitives::phrase(&["that", "permanent"]),
                primitives::phrase(&["this", "permanent"]),
            )),
            eof,
        )
            .void(),
        "sacrifice reference",
    )
    .is_ok();
    Ok(SacrificeAtEndOfCombatShape {
        object_tokens,
        tagged_object,
    })
}

pub fn parse_sacrifice_at_end_of_combat_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SacrificeAtEndOfCombatShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_sacrifice_at_end_of_combat_lexed,
        "sacrifice at end of combat",
    )
    .ok()
}

fn put_or_remove_counter_kind_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "put", "another", "counter", "of", "that", "kind", "on", "it", "or", "remove", "one",
        "from", "it",
    ])
    .parse_next(input)
}

fn parse_for_each_counter_kind_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ForEachCounterKindShape<'a>> {
    primitives::phrase(&["for", "each", "kind", "of", "counter", "on"]).parse_next(input)?;
    let target_tokens = repeat_till(
        1..,
        any.void(),
        peek((opt(primitives::comma()), put_or_remove_counter_kind_tail)),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    put_or_remove_counter_kind_tail.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ForEachCounterKindShape { target_tokens })
}

pub fn parse_for_each_counter_kind_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ForEachCounterKindShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_for_each_counter_kind_lexed,
        "for each counter kind",
    )
    .ok()
}

fn fight_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("fight"), primitives::kw("fights")))
        .void()
        .parse_next(input)
}

fn get_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("get"), primitives::kw("gets")))
        .void()
        .parse_next(input)
}

fn parse_gets_then_fights_lexed<'a>(input: &mut LexStream<'a>) -> WResult<GetsThenFightsShape<'a>> {
    opt(primitives::kw("then")).parse_next(input)?;
    let pump_tokens = repeat_till(
        1..,
        any.void(),
        peek((opt(primitives::kw("and")), fight_word)),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    let first_target_tokens = primitives::parse_prefix(
        pump_tokens,
        repeat_till(1.., any.void(), peek(get_word))
            .map(|((), _)| ())
            .take(),
    )
    .map(|(target, _)| target)
    .ok_or_else(|| primitives::backtrack_err("gets then fights", "pump subject"))?;
    opt(primitives::kw("and")).parse_next(input)?;
    fight_word.parse_next(input)?;
    let second_target_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(GetsThenFightsShape {
        pump_tokens,
        first_target_tokens,
        second_target_tokens,
    })
}

pub fn parse_gets_then_fights_tokens(tokens: &[OwnedLexToken]) -> Option<GetsThenFightsShape<'_>> {
    primitives::parse_all(tokens, parse_gets_then_fights_lexed, "gets then fights").ok()
}

fn parse_draw_then_connive_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DrawThenConniveShape<'a>> {
    let draw_tokens = repeat_till(1.., any.void(), peek(primitives::comma()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::kw("then").parse_next(input)?;
    let connive_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(DrawThenConniveShape {
        draw_tokens,
        connive_tokens,
    })
}

pub fn parse_draw_then_connive_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DrawThenConniveShape<'_>> {
    primitives::parse_all(tokens, parse_draw_then_connive_lexed, "draw then connive").ok()
}

fn additional_descriptor_on_tagged<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterDescriptorShape> {
    let descriptor = parse_counter_descriptor_lexed
        .verify(|descriptor| descriptor.additional)
        .parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    tagged_reference.parse_next(input)?;
    Ok(descriptor)
}

fn supported_enters_predicate<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        (
            opt(primitives::kw("a")),
            primitives::phrase(&["creature", "enters", "this", "way"]),
        )
            .void(),
        (
            primitives::phrase(&["it", "enters", "as"]),
            opt(primitives::kw("a")),
            primitives::kw("creature"),
        )
            .void(),
    ))
    .parse_next(input)
}

fn parse_if_enters_additional_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AdditionalCounterShape> {
    primitives::kw("if").parse_next(input)?;
    supported_enters_predicate.parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "enters", "with"]).parse_next(input)?;
    let descriptor = additional_descriptor_on_tagged.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(AdditionalCounterShape { descriptor })
}

pub fn parse_if_enters_additional_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AdditionalCounterShape> {
    primitives::parse_all(
        tokens,
        parse_if_enters_additional_lexed,
        "if enters with additional counter",
    )
    .ok()
}

fn tagged_enters_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["each", "of", "them", "enters", "with"]),
        primitives::phrase(&["each", "enters", "with"]),
        primitives::phrase(&["all", "of", "them", "enter", "with"]),
        primitives::phrase(&["that", "card", "enters", "with"]),
        primitives::phrase(&["that", "creature", "enters", "with"]),
        primitives::phrase(&["that", "planeswalker", "enters", "with"]),
        primitives::phrase(&["that", "object", "enters", "with"]),
        primitives::phrase(&["that", "permanent", "enters", "with"]),
        primitives::phrase(&["it", "enters", "with"]),
    ))
    .parse_next(input)
}

fn parse_tagged_enters_additional_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AdditionalCounterShape> {
    tagged_enters_prefix.parse_next(input)?;
    let descriptor = additional_descriptor_on_tagged.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(AdditionalCounterShape { descriptor })
}

pub fn parse_tagged_enters_additional_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AdditionalCounterShape> {
    primitives::parse_all(
        tokens,
        parse_tagged_enters_additional_lexed,
        "tagged enters with additional counter",
    )
    .ok()
}

fn conditional_entry_object_type<'a>(input: &mut LexStream<'a>) -> WResult<CardType> {
    alt((
        primitives::kw("artifact").value(CardType::Artifact),
        primitives::kw("battle").value(CardType::Battle),
        primitives::kw("creature").value(CardType::Creature),
        primitives::kw("enchantment").value(CardType::Enchantment),
        primitives::kw("instant").value(CardType::Instant),
        primitives::kw("land").value(CardType::Land),
        primitives::kw("planeswalker").value(CardType::Planeswalker),
        primitives::kw("sorcery").value(CardType::Sorcery),
    ))
    .parse_next(input)
}

fn conditional_entry_counter_arm<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ConditionalEntryCounterArmShape> {
    let descriptor = additional_descriptor_on_tagged.parse_next(input)?;
    primitives::kw("if").parse_next(input)?;
    alt((
        primitives::kw("it's").void(),
        primitives::kw("it’s").void(),
        primitives::phrase(&["it", "is"]).void(),
    ))
    .parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    let object_type = conditional_entry_object_type.parse_next(input)?;
    Ok(ConditionalEntryCounterArmShape {
        descriptor,
        object_type,
    })
}

fn parse_tagged_conditional_entry_counters_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TaggedConditionalEntryCountersShape> {
    primitives::phrase(&["each", "of", "them", "enters", "with"]).parse_next(input)?;
    let first = conditional_entry_counter_arm.parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let second = conditional_entry_counter_arm.parse_next(input)?;
    let remaining: Vec<ConditionalEntryCounterArmShape> = repeat(
        0..,
        (primitives::kw("and"), conditional_entry_counter_arm).map(|(_, arm)| arm),
    )
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let mut arms = vec![first, second];
    arms.extend(remaining);
    Ok(TaggedConditionalEntryCountersShape { arms })
}

pub fn parse_tagged_conditional_entry_counters_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TaggedConditionalEntryCountersShape> {
    primitives::parse_all(
        tokens,
        parse_tagged_conditional_entry_counters_lexed,
        "tagged conditional entry counters",
    )
    .ok()
}

fn move_onto_battlefield<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek((
            primitives::kw("onto"),
            opt(primitives::kw("the")),
            primitives::kw("battlefield"),
        )),
    )
    .void()
    .parse_next(input)?;
    (
        primitives::kw("onto"),
        opt(primitives::kw("the")),
        primitives::kw("battlefield"),
    )
        .void()
        .parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., any.void()).parse_next(input)?;
    eof.void().parse_next(input)
}

fn parse_put_with_additional_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PutWithAdditionalCounterShape<'a>> {
    let move_tokens = (
        primitives::kw("put"),
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("with"))).void(),
    )
        .take()
        .verify(|tokens: &&[OwnedLexToken]| {
            primitives::parse_all(tokens, move_onto_battlefield, "move onto battlefield").is_ok()
        })
        .parse_next(input)?;
    primitives::kw("with").parse_next(input)?;
    let descriptor = additional_descriptor_on_tagged.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PutWithAdditionalCounterShape {
        move_tokens,
        descriptor,
    })
}

pub fn parse_put_with_additional_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PutWithAdditionalCounterShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_put_with_additional_lexed,
        "put with additional counter",
    )
    .ok()
}

fn sacrifice_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("sacrifice"), primitives::kw("sacrifices")))
        .void()
        .parse_next(input)
}

fn parse_sacrifice_then_put_additional_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SacrificeThenPutAdditionalShape<'a>> {
    let sacrifice_tokens = (
        sacrifice_word,
        repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek((primitives::kw("then"), primitives::kw("put"))),
        )
        .void(),
    )
        .take()
        .parse_next(input)?;
    primitives::kw("then").parse_next(input)?;
    let put = parse_put_with_additional_lexed.parse_next(input)?;
    let plain_word_sacrifice = primitives::parse_all(
        sacrifice_tokens,
        (
            sacrifice_word,
            repeat::<_, _, (), _, _>(
                1..,
                any.verify(|token: &&OwnedLexToken| token.as_word().is_some())
                    .void(),
            ),
            eof,
        )
            .void(),
        "plain sacrifice",
    )
    .is_ok();
    Ok(SacrificeThenPutAdditionalShape {
        sacrifice_tokens,
        plain_word_sacrifice,
        put,
    })
}

pub fn parse_sacrifice_then_put_additional_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SacrificeThenPutAdditionalShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_sacrifice_then_put_additional_lexed,
        "sacrifice then put with additional counter",
    )
    .ok()
}

fn parse_if_sacrifice_then_put_additional_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<IfSacrificeThenPutAdditionalShape<'a>> {
    primitives::kw("if").parse_next(input)?;
    let predicate_tokens = repeat_till(1.., any.void(), peek(sacrifice_word))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let effect = parse_sacrifice_then_put_additional_lexed.parse_next(input)?;
    Ok(IfSacrificeThenPutAdditionalShape {
        predicate_tokens,
        effect,
    })
}

pub fn parse_if_sacrifice_then_put_additional_tokens(
    tokens: &[OwnedLexToken],
) -> Option<IfSacrificeThenPutAdditionalShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_if_sacrifice_then_put_additional_lexed,
        "if sacrifice then put with additional counter",
    )
    .ok()
}

fn each_player_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["for", "each", "player"]),
        primitives::phrase(&["for", "each", "players"]),
        primitives::phrase(&["each", "player"]),
        primitives::phrase(&["each", "players"]),
    ))
    .parse_next(input)
}

fn return_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("return"), primitives::kw("returns")))
        .void()
        .parse_next(input)
}

fn parse_each_player_return_additional_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EachPlayerReturnAdditionalShape<'a>> {
    each_player_prefix.parse_next(input)?;
    let return_tokens = (
        return_word,
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("with"))).void(),
    )
        .take()
        .parse_next(input)?;
    primitives::kw("with").parse_next(input)?;
    let descriptor = additional_descriptor_on_tagged.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(EachPlayerReturnAdditionalShape {
        return_tokens,
        descriptor,
    })
}

pub fn parse_each_player_return_additional_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EachPlayerReturnAdditionalShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_each_player_return_additional_lexed,
        "each player return with additional counter",
    )
    .ok()
}

fn choice_counter_type<'a>(input: &mut LexStream<'a>) -> WResult<CounterType> {
    opt(alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("one"),
    )))
    .parse_next(input)?;
    let counter_type = alt((
        primitives::phrase(&["first", "strike"]).value(CounterType::FirstStrike),
        primitives::phrase(&["double", "strike"]).value(CounterType::DoubleStrike),
        choice_single_counter_type,
    ))
    .parse_next(input)?;
    opt(counter_noun).parse_next(input)?;
    Ok(counter_type)
}

fn choice_single_counter_type<'a>(input: &mut LexStream<'a>) -> WResult<CounterType> {
    let token: &OwnedLexToken = any.parse_next(input)?;
    crate::util::parse_counter_type_word(token.parser_text())
        .ok_or_else(|| primitives::backtrack_err("counter choice", "recognized counter type"))
}

fn parse_put_counter_choice_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PutCounterChoiceShape<'a>> {
    // "Put your choice of A, B, or C on ..." or the bare or-joined form
    // "Put a +0/+1 counter or a +1/+0 counter on ..." — an "or" between
    // counters is always a mode choice in oracle templating.
    alt((
        primitives::phrase(&["put", "your", "choice", "of"]).void(),
        primitives::kw("put").void(),
    ))
    .parse_next(input)?;
    let counter_types =
        separated(2.., choice_counter_type, primitives::comma_or_separator).parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    let target_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PutCounterChoiceShape {
        counter_types,
        target_tokens,
    })
}

pub fn parse_put_counter_choice_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PutCounterChoiceShape<'_>> {
    primitives::parse_all(tokens, parse_put_counter_choice_lexed, "put counter choice").ok()
}

fn parse_put_fixed_and_counter_choice_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PutFixedAndCounterChoiceShape<'a>> {
    primitives::kw("put").parse_next(input)?;
    let fixed = parse_counter_descriptor_lexed(input)?;
    primitives::phrase(&["and", "a", "counter", "from", "among"]).parse_next(input)?;
    let counter_types =
        separated(2.., choice_counter_type, primitives::comma_or_separator).parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    let target_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PutFixedAndCounterChoiceShape {
        fixed,
        counter_types,
        target_tokens,
    })
}

pub fn parse_put_fixed_and_counter_choice_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PutFixedAndCounterChoiceShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_put_fixed_and_counter_choice_lexed,
        "put fixed and counter choice",
    )
    .ok()
}

fn contains_counter_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(counter_noun))
        .void()
        .parse_next(input)?;
    counter_noun.parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., any.void()).parse_next(input)?;
    eof.void().parse_next(input)
}

fn parse_put_counter_then_sequence_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PutCounterSequenceShape<'a>> {
    let head_tokens = (
        primitives::kw("put"),
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("then"))).void(),
    )
        .take()
        .verify(|tokens: &&[OwnedLexToken]| {
            primitives::parse_all(tokens, contains_counter_noun, "counter sequence head").is_ok()
        })
        .parse_next(input)?;
    primitives::kw("then").parse_next(input)?;
    let tail_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PutCounterSequenceShape::Then {
        head_tokens,
        tail_tokens,
    })
}

fn parse_plain_put_counter_sequence_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PutCounterSequenceShape<'a>> {
    let tokens = (
        primitives::kw("put"),
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .void(),
    )
        .take()
        .verify(|tokens: &&[OwnedLexToken]| {
            primitives::parse_all(tokens, contains_counter_noun, "counter sequence").is_ok()
        })
        .parse_next(input)?;
    let _ = tokens;
    primitives::sentence_end().parse_next(input)?;
    Ok(PutCounterSequenceShape::Plain)
}

pub fn parse_put_counter_sequence_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PutCounterSequenceShape<'_>> {
    primitives::parse_all(
        tokens,
        alt((
            parse_put_counter_then_sequence_lexed,
            parse_plain_put_counter_sequence_lexed,
        )),
        "put counter sequence",
    )
    .ok()
}

fn counter_placement<'a>(input: &mut LexStream<'a>) -> WResult<CounterPlacementShape<'a>> {
    let descriptor = parse_counter_descriptor_lexed.parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    let target_tokens = repeat_till(
        1..,
        any.void(),
        peek(alt((
            primitives::comma().void(),
            (primitives::kw("and"), peek(parse_counter_descriptor_lexed)).void(),
            primitives::sentence_end(),
        ))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    Ok(CounterPlacementShape {
        descriptor,
        target_tokens,
    })
}

fn parse_counter_placement_sequence_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<Vec<CounterPlacementShape<'a>>> {
    primitives::kw("put").parse_next(input)?;
    let first = counter_placement.parse_next(input)?;
    let rest: Vec<CounterPlacementShape<'a>> = repeat(
        1..,
        (
            alt((
                (primitives::comma(), opt(primitives::kw("and"))).void(),
                primitives::kw("and").void(),
            )),
            counter_placement,
        )
            .map(|(_, placement)| placement),
    )
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let mut placements = Vec::with_capacity(rest.len() + 1);
    placements.push(first);
    placements.extend(rest);
    Ok(placements)
}

pub fn parse_counter_placement_sequence_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<CounterPlacementShape<'_>>> {
    primitives::parse_all(
        tokens,
        parse_counter_placement_sequence_lexed,
        "counter placement sequence",
    )
    .ok()
}

fn descriptor_separator<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        (primitives::comma(), opt(primitives::kw("and"))).void(),
        primitives::kw("and").void(),
    ))
    .parse_next(input)
}

fn parse_shared_counter_target_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SharedCounterTargetShape<'a>> {
    primitives::kw("put").parse_next(input)?;
    let descriptors =
        separated(2.., parse_counter_descriptor_lexed, descriptor_separator).parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    let target_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(SharedCounterTargetShape {
        descriptors,
        target_tokens,
    })
}

pub fn parse_shared_counter_target_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SharedCounterTargetShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_shared_counter_target_lexed,
        "shared counter target",
    )
    .ok()
}

fn parse_counter_followup_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterFollowupShape<'a>> {
    primitives::kw("put").parse_next(input)?;
    let counter_tokens = repeat_till(1.., any.void(), peek(primitives::phrase(&["and", "it"])))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::phrase(&["and", "it"]).parse_next(input)?;
    let followup_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterFollowupShape {
        counter_tokens,
        followup_tokens,
    })
}

pub fn parse_counter_followup_tokens(tokens: &[OwnedLexToken]) -> Option<CounterFollowupShape<'_>> {
    primitives::parse_all(tokens, parse_counter_followup_lexed, "counter followup").ok()
}

fn parse_counter_pair_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CounterPairShape<'a>> {
    primitives::kw("put").parse_next(input)?;
    let first_tokens = repeat_till(1.., any.void(), peek(primitives::kw("and")))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let second_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterPairShape {
        first_tokens,
        second_tokens,
    })
}

pub fn parse_counter_pair_tokens(tokens: &[OwnedLexToken]) -> Option<CounterPairShape<'_>> {
    primitives::parse_all(tokens, parse_counter_pair_lexed, "counter pair").ok()
}

#[cfg(test)]
#[path = "counter_marker_shapes_inline_tests.rs"]
mod tests;
